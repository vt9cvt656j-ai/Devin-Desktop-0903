# Michael IDE 智能体重构设计(方案 B:六阶段换心脏)

日期:2026-07-20
状态:已获批准(方案 B + 每阶段测试通过后立即部署生产)
基线:`codex/production-recovery-and-fixes` @ a3a9ee2(v0.3.15,已回滚未提交的 cognition-contract 重构,存档于 `backup/cognition-refactor-20260720`)

## 1. 背景与诊断

用户对内置智能体的六项核心不满:回答质量差、光说不做、改代码不可靠、慢卡断流、不了解项目、过程不透明。

两路架构审查(前端 `ide/src/main.js` 4.5 万行 + 服务端 `server/`)定位出六大结构性病根:

| # | 病根 | 证据 |
|---|---|---|
| 1 | 系统提示组装后约 2 万 token,指令自相矛盾(禁列表 vs 全文表格;别乱调工具 vs 必须组合多工具;禁思考卡片 vs 强制推理检查点) | `server/prompts/agent.txt` 426 行;`prompts.rs::assemble_into` 1505-1870 |
| 2 | 提示词教模型调用不存在的工具(`academic_search`、`cve_search`、`automation`、`capture_*`、`search_tools` 等),造成失败轮次与空转 | `agent.txt`/`agent_lite.txt` vs `tools.json`(131 个真实工具) |
| 3 | 工具面过宽:前端 ~148 个工具、37k token schema;UI 设计栈(shadcn 四件套)对 agent/plan 模式无条件注入,修后端也吃前端设计宪法 | `_buildAgentToolSchemas` 20716-20893;`prompts.rs` 1652-1678 |
| 4 | 主循环是 1674 行的"门禁/催促机"而非干净的检索→编辑→验证循环:效果合同、工具优先 nudge、UI 验证矩阵、卡住检测等合成消息淹没真实用户意图 | `_runAgenticLoop` 28610-30284 |
| 5 | 上下文靠字节裁剪而非语义管理:目录树 3 层 180 行、git 状态默认不注入、BM25 无语义索引、长任务折叠早期证据成指针 | `_gatherAgentContext` 15081-15183;`_trimMessagesIfHuge` 24318+ |
| 6 | 缓存与推理自毁:网关 `strip_cache_control` 剥掉前缀缓存;推理检查点每轮改写 user 消息尾部破坏消息哈希;思考深度不随任务难度自适应 | `models.rs` 1908-1937;`prompts.rs` 1684-1774 |

一句话:现状用"工具数量 × 提示词长度 × 门禁数量"堆智能;目标改为"精准上下文 × 小工具集 × 干净循环"。

## 2. 目标与非目标

**目标**(与六痛点一一对应):

1. 系统提示总量降到 ≤5k token(行为宪法 ≤3k + 按需领域块),指令零冲突 → 答得准
2. 提示词工具名与 `tools.json` 机器对齐(测试强制) → 不空转
3. 默认工具面 ≤25 个编码核心工具,垂直工具按需挂载 → 注意力集中
4. 主循环干净化:移除正则门禁与合成催促,统一错误恢复协议 → 改代码可靠
5. 默认上下文含 git status/diff、全仓诊断、深目录树;分层摘要替代字节裁剪 → 懂项目
6. 前缀缓存恢复 + 消息哈希稳定 → 快、便宜;思考流照常展示 → 过程透明

**非目标**:不换 UI 框架、不动登录/计费/支付逻辑、不换模型供应商体系、不改扩展系统。

## 3. 六阶段设计

每阶段独立分支意义上的"可验收单元":测试全过 + 真机实测 + git 提交 + (涉服务端时)部署生产。任一阶段翻车,单独回滚该阶段提交,不牵连其他。

### 阶段一:服务端提示词体系重建(先做,收益最大)

- `agent.txt` 426 行 → 分层:
  - L0 身份 + 完成定义(≤500 字)
  - L1 编码执行循环:检索→计划→编辑→验证→收尾(核心)
  - L2 证据与真实性纪律(合并 `truthfulness.txt`/`answer_quality.txt` 重复段)
  - L3 领域块(research / UI / automation)改意图门控,默认不挂
- 删除所有幻觉工具名;`prompts.rs` 增加测试:提示词中出现的工具名必须 ⊆ `tools.json`
- UI 四件套从"agent/plan 无条件注入"改回意图门控
- 移除推理检查点在 user 消息尾部的双写(保留/简化系统侧一处,或交给模型原生 thinking)
- `agent_lite.txt` 同步重写(弱模型版,同一宪法的压缩版)
- 更新 `prompts.rs` 中断言旧提示词原文的测试
- 验收:`cargo test` 全过;部署后真机对话质量肉眼可感;网关日志 `prompt_blocks` 体积下降

### 阶段二:工具面收缩

- 网关侧:agent 默认注入编码核心工具白名单(read/write/edit/search/list/terminal/git/lsp/diagnostics/browser/http/plan/task_state/ask_user 等 ≤25 个)
- 垂直搜索/游戏/购物等工具保留实现,归入按需 bundle,由意图或显式请求挂载
- 超长工具描述瘦身(browser/figma 等 >900 字的压缩到 ≤300 字)
- 前端 `_TOOL_PAYLOAD_MAX_TOOLS` 等预算相应收紧
- 验收:默认请求工具 schema 字节数下降 ≥70%;编码任务实测工具选型命中率提升

### 阶段三:main.js 拆模块(纯搬家,不改行为)

- 抽出 `ide/src/agent/`:`loop.js`(主循环)、`model-turn.js`(SSE 消费/tool_calls 解析)、`tools/schema.js`、`tools/execute.js`、`tools/parallel.js`、`context/gather.js`、`context/budget.js`、`prompts.js`、`plan.js`;`conversation-memory.js` 归位 `agent/memory/`
- 只移动 + import,零行为变更;`node --test` 286 个用例全过作为搬家不破坏的证据
- 验收:`npm test` 全过 + `npm run tauri dev` 启动 + 真机冒烟

### 阶段四:主循环重写

- `_runAgenticLoop` 1674 行 → 干净循环:组装上下文 → 模型轮 → 执行工具(只读并行/写串行)→ 观察结果 → 循环;停止条件:模型自然收尾/预算/用户取消
- 拆除:效果合同正则门禁、工具优先 nudge、UI 验证矩阵、卡住 nudge 链;保留:危险命令确认、写前必读、步数预算等少量硬安全线
- 错误恢复统一为一层 tool-result 协议(错误分类:可重试/需修参/需换路/需问人)
- 验收:新增循环单测(用 mock 模型驱动);真机跑"改文件+跑测试"全链路;对照六痛点逐项实测

### 阶段五:上下文协议升级

- 默认注入:git status/diff 摘要、全仓 LSP 诊断汇总、目录树加深(自适应预算)、最近编辑文件账本
- 长任务:分层摘要(任务级里程碑 + 文件证据账本)替代纯字节折叠
- 验收:真机在中型仓库发"不带任何上下文的任务",智能体能自主定位到正确文件

### 阶段六:Plan 状态机 + 跨会话记忆

- update_plan 升级为一等任务状态机(步骤/状态/证据),UI 时间线可视
- 跨会话项目记忆(关键决策/约定/坑)自动沉淀与召回
- 验收:中断任务可恢复;第二次会话不重复踩同一坑

### 部署与回滚

- 服务端阶段(一、二):`server/deploy.sh` 部署 code.mrday.one,自带预部署备份(/var/backups/michael-ide);回滚 = git revert + 重部署
- 前端阶段(三~六):本地 `npm run tauri dev` 验收;每阶段一个 commit,回滚 = revert 单个提交

## 4. 风险

| 风险 | 缓解 |
|---|---|
| 重写提示词后行为回归无自动化衡量 | 每阶段固定一组真机验收任务(改文件/查项目/跑命令/长任务),前后对比 |
| prompts.rs 旧测试断言原文,重写即红 | 阶段一同步重写测试,断言"结构与不变量"而非原文 |
| 拆 main.js 引入隐性循环依赖 | 纯搬家阶段禁止任何逻辑修改,靠测试+冒烟兜底 |
| 生产部署风险 | 每次部署前自动备份;健康检查失败自动显示日志 |
