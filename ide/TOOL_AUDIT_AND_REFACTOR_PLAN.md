# Michael IDE 工具全面审计报告

> 审计时间：2026-07-31
> 审计范围：155 个 schema 定义工具 + 7 个 deferred 工具（game_scaffold/web_scaffold/generate_3d/generate_sound/generate_music/generate_voice/auto_rig/generate_motion/generate_texture/search_game_assets/download_asset）
> 主文件：`src/main.js`（56601 行）+ `src/tool-guides.js`（513 行）
> 审计方式：只读，不改代码

---

## 一、总体结论

### 1.1 工具总数与分布

| 类别 | 数量 | 评分均值 |
|------|------|----------|
| 文件操作 | 10 | 4.2 |
| 搜索定位 | 5 | 4.0 |
| 执行验证 | 6 | 4.3 |
| Git 版本控制 | 14 | 4.0 |
| GitHub/CI/PR | 6 | 3.8 |
| 浏览器/UI 自动化 | 6 | 3.5 |
| 网络/HTTP | 4 | 3.8 |
| 数据库 | 1 | 3.5 |
| 知识检索/LSP | 7 | 3.7 |
| 调研搜索类（deferred） | ~60 | 3.0 |
| 设计/创意 | 8 | 3.2 |
| 子智能体编排 | 5 | 4.0 |
| 地理/生活/商业 | 8 | 3.5 |
| 游戏/3D 资产 | 10 | 2.8 |
| 部署/脚手架 | 4 | 3.5 |
| 其他（capture/automation/monitor） | 8 | 3.3 |
| 规划/记忆/交互 | 5 | 4.2 |

### 1.2 核心发现

**优势（已做得好的）：**
- **核心文件操作工具（read_file/edit_file/write_file/find_files）** 实现极其健壮，有路径兜底、模糊匹配、失败渐进门控、内容签名去重、冗余读拦截等机制
- **run_cmd** 有失败命令重跑短路（L28615）、命令风险分级、超时控制
- **工具别名系统**（L25897-25982）极其完善，弱模型拼写错误自动纠正
- **参数校验层**（_toolArgIssue/_schemaValueIssue/_coerceSchemaTypes）多层防御，类型自愈
- **Tool Ledger**（L35825-35853）完整记录每次工具调用的成功/失败，跨会话经验沉淀
- **搜索去重**（L37126-37140）防止模型脑冻结重复搜索
- **写入门禁**（L40421-40446）路径合法性检查、同批次读写绑定

**主要问题：**
- **60+ 个 deferred 搜索工具** 全部走统一泛化执行路径（L42965-42988），无独立失败记忆、无结果缓存、无"何时用"描述
- **新工具（#10/#11）** 实现完整但缺乏与基座协同的失败记忆和去重
- **Git 工具** 无失败记忆，git push/clone 失败后模型可无限重试
- **Browser 工具** 实现健壮但缺乏与子智能体的协同（浏览器锁机制可能阻塞子智能体）
- **db_query** 有连接重试但无查询失败记忆

---

## 二、按能力域分类的详细审计

### 2.1 文件操作工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| read_file | 5 | L25333 | L40488-40850 | ✅ 极其健壮：路径多根候选、模糊恢复、内容签名去重、冗余读拦截、失败渐进门控(#79)、目录自动转list_dir |
| write_file | 4 | L25418 | L40906-40993 | ✅ 路径绑定、空内容拦截、写前读取绑定检查。⚠️ 无写入失败记忆（写入失败后模型可反复尝试同一路径） |
| edit_file | 4 | L25416 | L40995-41100 | ✅ old_string 模糊恢复(_recoverEditMatch)、不存在文件路径提示。⚠️ 无编辑失败记忆 |
| multi_edit | 4 | L25417 | L41100+ | ✅ 原子应用、单处失败整体不写入。⚠️ 无编辑失败记忆 |
| list_dir | 4 | L25334 | L40553-40581 | ✅ 空目录检测、上级目录提示。⚠️ 无失败记忆 |
| delete_path | 3 | L25421 | ~L41200 | ⚠️ 无确认机制、无失败记忆、无回收站 |
| move_path | 3 | L25422 | ~L41200 | ⚠️ 无目标冲突检测、无失败记忆 |
| copy_path | 3 | L25428 | ~L41200 | ⚠️ 无覆盖确认、无失败记忆 |
| create_dir | 3 | L25428 | ~L41200 | ⚠️ 无存在性检查、无失败记忆 |
| format_file | 3 | L25430 | ~L41200 | ⚠️ 无格式化失败回退、无失败记忆 |

**基座协同评估：**
- ✅ read_file 有 `_failedPathAttempts` 失败记忆（#79）
- ✅ 有 `_dupReadN`/`_seenRead` 去重机制
- ✅ 有内容签名 `_contentSignature` 检测真实变化
- ⚠️ write/edit/multi_edit 无失败记忆，写入失败后不会递增门控

### 2.2 搜索定位工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| search | 4 | L25335 | L41358-41416 | ✅ 上下文可选、regex/literal 模式。✅ 有 `_failedSearchQ` 失败搜索记忆（L37145-37260）。⚠️ 搜索结果无缓存 |
| find_files | 5 | L25336 | L30975-31023 | ✅ 刚重写(#81)：用 `_agentDirEntryName`/`_agentDirEntryIsDir` 健壮 helper、IGNORED 集合、MAX 限制、失败计数递增(#79) |
| find_symbol | 4 | L25406 | ~L42000 | ✅ 跨工程符号查找。⚠️ 无失败记忆 |
| semantic_search | 4 | L25407 | ~L42000 | ✅ 语义搜索。⚠️ 无失败记忆、无结果缓存 |
| knowledge_search | 4 | L25408 | ~L42000 | ✅ 知识库检索。⚠️ 无失败记忆 |

**基座协同评估：**
- ✅ search 有 `_failedSearchQ` 相似搜索去重（L37145-37162）
- ✅ find_files 有 `_failedPathAttempts` 递增门控
- ⚠️ find_symbol/semantic_search/knowledge_search 无失败记忆

### 2.3 执行验证工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| run_cmd | 5 | L25419 | L44190-44300 | ✅ 失败命令重跑短路(#29)、命令风险分级、超时控制、子智能体60s超时、验证命令识别 |
| run_in_terminal | 4 | L25419附近 | L42773-42850 | ✅ 服务型命令自动ready检测、终端管理。⚠️ 无启动失败记忆 |
| read_terminal | 3 | L25403 | ~L42800 | ⚠️ 简单实现，无失败记忆 |
| list_terminals | 3 | L25404 | ~L42800 | ⚠️ 简单列出，无问题 |
| stop_terminal | 3 | - | ~L42800 | ⚠️ 简单实现 |
| get_diagnostics | 4 | L25388 | L41433-41466 | ✅ 依赖解析诊断自动刷新缓存、格式化输出。⚠️ 无诊断失败记忆 |

**基座协同评估：**
- ✅ run_cmd 有 `_repeatedFailedCmdShortCircuit`（L28615）失败命令拦截
- ✅ 有命令风险分级 `_commandRiskKind`
- ✅ 有验证命令识别 `_looksLikeVerificationCommand`
- ⚠️ run_in_terminal 无启动失败记忆

### 2.4 Git 版本控制工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| git_status | 4 | L25390 | L42460+ | ✅ 仓库上下文自动检测、非仓库引导 |
| git_diff | 4 | L25391 | L42460+ | ✅ staged/unstaged 区分 |
| git_log | 3 | L25392 | L42460+ | ⚠️ 无失败记忆 |
| git_commit | 4 | L25423 | L42460+ | ✅ 默认 add -A、只读模式拦截 |
| git_branch | 3 | L25424 | L42460+ | ⚠️ 无分支名合法性检查、无失败记忆 |
| git_push | 3 | L25425 | L42460+ | ⚠️ 无凭据失败快速处理记忆、无失败记忆 |
| git_clone | 3 | L25426 | L42460+ | ⚠️ 无目标路径冲突检测、无失败记忆 |
| git_pull | 3 | L25427 | L42460+ | ⚠️ 无合并冲突自动检测、无失败记忆 |
| git_blame | 3 | L25393 | L42460+ | ⚠️ 无失败记忆 |
| git_stash | 3 | - | L42460+ | ⚠️ 无失败记忆 |
| git_stash_pop | 3 | - | L42460+ | ⚠️ 无冲突检测、无失败记忆 |
| git_stash_list | 3 | L25394 | L42460+ | ⚠️ 简单列出 |
| git_conflicts | 3 | L25395 | L42460+ | ⚠️ 无失败记忆 |
| deploy_site | 4 | L25420 | L26576-26584 | ✅ 安全部署：JWT鉴权+白名单+限大小+安全解压+隔离 |

**基座协同评估：**
- ✅ Git 工具有仓库上下文检测 `_gitResolveRepoContext`
- ✅ 有只读模式拦截
- ⚠️ **所有 Git 工具均无失败记忆**——git push 失败后模型可无限重试同一操作
- ⚠️ Git 工具无结果缓存（git status 每次重新执行是合理的，但 git log 可缓存）

### 2.5 GitHub/CI/PR 工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| gh_pr_create | 4 | L25397 | L42460+ | ✅ 有外部效果分类。⚠️ 无失败记忆 |
| gh_pr_view | 3 | L25398 | L42460+ | ⚠️ 无结果缓存 |
| gh_pr_checks | 3 | L25399 | L42460+ | ⚠️ 无结果缓存、无失败记忆 |
| gh_actions_log | 3 | L25400 | L42460+ | ⚠️ 无结果缓存 |
| gh_pr_review_comments | 3 | L25401 | L42460+ | ⚠️ 无结果缓存 |
| gh_pr_reply | 3 | L25402 | L42460+ | ⚠️ 无失败记忆 |

### 2.6 浏览器/UI 自动化

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| browser | 4 | L25342附近 | L43846-43900 | ✅ 有浏览器锁机制(_browserAgentOwner)、多 run 隔离、force close。⚠️ 锁机制可能阻塞子智能体 |
| screenshot | 4 | L25411 | L42178+ | ✅ 逐帧胶片模式、URL验证。⚠️ 无失败记忆 |
| read_screen | 3 | L25348 | ~L43000 | ⚠️ 桌面专用、无失败记忆 |
| ui_click | 3 | L25349 | ~L43000 | ⚠️ 桌面专用、无失败记忆 |
| visual_compare | 3 | L25412 | ~L43000 | ⚠️ 桌面专用 |
| performance_profile | 3 | L25342 | L43625-43700 | ✅ 有超时控制、localhost验证。⚠️ 仅支持localhost、无失败记忆 |

**基座协同评估：**
- ✅ browser 有 run 级别锁（_browserAgentOwner）
- ✅ 有 #21 熔断机制
- ⚠️ 浏览器锁与子智能体协同不够——子智能体无法使用浏览器

### 2.7 网络/HTTP 工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| web_fetch | 4 | L25338 | L42100-42150 | ✅ 有网络重试 `_withNetworkRetry`、结果缓存 `_webCachePut`。⚠️ 缓存无过期 |
| web_search | 4 | L25337 | L42153-42176 | ✅ 有结果缓存 `_agentWebCache`、自动深读 `_autoDeepRead`。⚠️ 无搜索失败记忆 |
| http_request | 4 | - | L42895-42925 | ✅ 有网络重试、幂等方法自动重试、redirect 检测。⚠️ 无失败记忆 |
| tor_request | 3 | - | L42927-42947 | ✅ 桌面专用检查。⚠️ 无失败记忆 |
| download_file | 3 | - | ~L43000 | ⚠️ 无进度反馈、无失败记忆 |

### 2.8 数据库工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| db_query | 3.5 | - | L43207-43231 | ✅ 有连接重试 `_dbQueryWithRetry`（L44644-44659）、连接类错误自动重试1次。⚠️ 无查询失败记忆、无连接池管理、无慢查询警告 |

### 2.9 知识检索/LSP 工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| lsp_symbols | 3 | L25405 | ~L42000 | ⚠️ 依赖 Monaco/LSP、无失败记忆 |
| lsp_definition | 3 | L25409 | ~L42000 | ⚠️ 无失败记忆 |
| lsp_references | 3 | L25410 | ~L42000 | ⚠️ 无失败记忆 |
| semantic_search | 4 | L25407 | ~L42000 | ✅ 按语义找代码。⚠️ 无失败记忆、无结果缓存 |
| knowledge_search | 4 | L25408 | ~L42000 | ✅ 知识库检索。⚠️ 无失败记忆 |
| recall_conversation | 3 | L25386 | ~L42000 | ⚠️ 无失败记忆 |
| remember | 3 | L25387 | ~L42000 | ⚠️ 无写入确认 |

### 2.10 调研搜索类工具（~60 个 deferred 工具）

**统一执行路径：** L42965-42988

所有 deferred 搜索工具（academic_search, package_search, github_search, github_repo, gitlab_repo, gitee_repo, codeberg_repo, cve_search, wiki_search, stackoverflow_search, hackernews_search, developer_community_search, dockerhub_search, pubmed_search, arxiv_search, crossref_search, openalex_search, pubchem_search, clinical_trials_search, gitlab_search, gitee_search, maven_search, packagist_search, rubygems_search, nuget_search, homebrew_search, mdn_search, cdnjs_search, bundlephobia_search, devto_search, reddit_search, steam_search, iconify_search, color_search, lobsters_search, juejin_search, codrops_search, smashingmag_search, css_tricks_search, codepen_search, dribbble_search, awwwards_search, v2ex_search, segmentfault_search, github_discussions_search, producthunt_search, freecodecamp_search, github_trending, infoq_search, hackernoon_search, codeberg_search, bestofjs_search, sourcegraph_search, deep_search, smzdm_search, xianyu_search, zhuanzhuan_search）共享同一个执行分支。

**统一特征：**
- ✅ 桌面专用检查
- ✅ 通过 `backend.invoke(call.type, _tauriSearchInvokeArgs(call))` 调用 Rust 后端
- ✅ 部分工具有自动深读 `_autoDeepRead`（`_DEEP_READ_SEARCHES` 集合内的工具）
- ✅ 结果截断保护（developer_community_search 35000字，其他 20000字）
- ⚠️ **无独立失败记忆**——所有搜索工具共享 `_failedSearchQ` 仅覆盖 search/websearch，不覆盖 deferred 搜索
- ⚠️ **无结果缓存**——每次调用都走网络
- ⚠️ **schema description 质量参差**——部分工具有"何时用"描述（如 web_search 的详尽替代工具列表），大部分只有功能描述

**Schema 描述质量评估：**
- 优秀（有"何时用"+替代工具）：web_search（L25337）、package_search（L25440）、github_search（L25441）
- 一般（只有功能描述）：大多数 *_search 工具
- 差（无使用指导）：smzdm_search, xianyu_search, zhuanzhuan_search 等

### 2.11 设计/创意工具

| 工具 | 评分 | 执行行号 | 问题 |
|------|------|----------|------|
| generate_image | 3 | ~L43000 | ⚠️ 桌面专用、无失败记忆 |
| design_board | 3 | ~L43000 | ⚠️ 桌面专用 |
| figma | 4 | L42949-42963 | ✅ 桌面/网页双端支持、完整错误处理。⚠️ 无失败记忆 |
| preview_choices | 3 | ~L43000 | ⚠️ 简单展示 |
| visual_explain | 3 | ~L43000 | ⚠️ 简单展示 |
| design_research | 4 | L25381 | ✅ 子智能体驱动 |
| learn_design | 4 | L25382 | ✅ 设计体系提取 |
| generate_wiki | 4 | L25383 | ✅ 子智能体驱动 |

### 2.12 子智能体编排工具

| 工具 | 评分 | Schema行号 | 执行行号 | 问题 |
|------|------|-----------|----------|------|
| run_subagent | 5 | L25377 | L33600+ | ✅ 多任务并发、角色分配、同步/异步模式、只读命令过滤、60s超时、failDigest回传 |
| await_subagent | 4 | L25378 | ~L33700 | ✅ 等待后台作业、台账摘要 |
| run_worker | 5 | L25384 | ~L33600 | ✅ scope 隔离、可写MCP拦截、文件路径绑定 |
| debate | 4 | L25379 | L41577-41650 | ✅ 多视角并行论证、裁判综合、超时控制 |
| research_project | 4 | L25380 | L26599 | ✅ 子智能体驱动的深度代码调研 |

**基座协同评估：**
- ✅ 子智能体工具有完整的隔离机制（scope/只读过滤/超时/failDigest）
- ✅ 有 worker scope guard（L40390-40409）
- ✅ 有并发任务管理

### 2.13 地理/生活/商业工具

| 工具 | 评分 | 问题 |
|------|------|------|
| local_discovery | 4 | ✅ 多源聚合、坐标验证。⚠️ 无失败记忆 |
| live_environment | 4 | ✅ 多源状态追踪。⚠️ 无失败记忆 |
| live_markets | 4 | ✅ 多源不冲突。⚠️ 无失败记忆 |
| live_flights | 3 | ⚠️ 覆盖有限、无失败记忆 |
| road_environment | 3 | ⚠️ 数据源有限、无失败记忆 |
| track_shipment | 3 | ⚠️ 只返回官方页面链接、无失败记忆 |
| shop_catalog | 3 | ⚠️ 依赖 Shopify/JSON-LD、无失败记忆 |
| realtime_news_feed | 3 | L44510 | ✅ 多源聚合(HN等)。⚠️ 无失败记忆、无结果缓存 |

### 2.14 游戏/3D 资产工具

| 工具 | 评分 | 执行行号 | 问题 |
|------|------|----------|------|
| game_scaffold | 3 | L41479-41493 | ✅ 桌面专用检查、工作区检查。⚠️ 无失败记忆 |
| web_scaffold | 3 | L41495-41520 | ✅ 自动接 learn_design tokens。⚠️ 无失败记忆 |
| generate_3d | 2 | L41522-41546 | ⚠️ 薄封装——直接透传 backend.invoke，无输入验证（prompt 可空）、无失败记忆 |
| generate_sound | 2 | L41522-41546 | ⚠️ 同上 |
| generate_music | 2 | L41522-41546 | ⚠️ 同上 |
| generate_voice | 2 | L41522-41546 | ⚠️ 同上 |
| auto_rig | 2 | L41522-41546 | ⚠️ 同上 |
| generate_motion | 2 | L41522-41546 | ⚠️ 同上 |
| generate_texture | 2 | L41522-41546 | ⚠️ 同上 |
| search_game_assets | 2 | L41548-41559 | ⚠️ 薄封装、无输入验证、无失败记忆 |
| download_asset | 2 | L41561-41575 | ⚠️ 薄封装、无失败记忆 |

### 2.15 新增工具（#10/#11）

| 工具 | 评分 | 执行行号 | 实现完整性 | 问题 |
|------|------|----------|-----------|------|
| generate_test_cases | 3.5 | L44341-44375 | ✅ 完整：读取文件→自动检测框架→提取导出目标→生成测试骨架。⚠️ 无失败记忆、无结果缓存 |
| docker_compose_up | 4 | L44376-44500 | ✅ 完整：安全校验（路径/服务名白名单）、v2/v1 自动探测、服务启动等待。⚠️ 无失败记忆 |
| performance_profile | 3 | L43625-43750 | ✅ 完整：浏览器注入Performance API、截图验证。⚠️ 仅支持localhost、无失败记忆 |
| openapi_parser | 3.5 | L43752-43845 | ✅ 完整：支持本地/远程、JSON/YAML解析、多输出格式。⚠️ 无失败记忆、无结果缓存 |
| realtime_news_feed | 3 | L44510-44600 | ✅ 完整：HN等多源聚合。⚠️ 源数量有限、无失败记忆、无结果缓存 |

### 2.16 其他工具

| 工具 | 评分 | 问题 |
|------|------|------|
| capture_start | 3 | ⚠️ 桌面专用、无失败记忆 |
| capture_flows | 3 | ⚠️ 桌面专用 |
| capture_stop | 3 | ⚠️ 简单停止 |
| capture_replay | 3 | ⚠️ 桌面专用 |
| background_monitor | 3.5 | ✅ 有超时控制。⚠️ 无失败记忆 |
| automation | 3 | ⚠️ 桌面专用、无失败记忆 |
| system | 3 | ⚠️ 桌面专用 |
| computer | 2 | ⚠️ 硬编码为 mouse.click（L26801），几乎无用 |
| current_time | 4 | ✅ 完整实现 |
| decode_qr | 3 | ⚠️ 桌面专用 |
| remote | 3 | ⚠️ 无连接失败记忆 |

---

## 三、需要重写的工具

### P0：完全不能用/严重缺陷（必须立即重写）

| 工具 | 问题 | 重写工作 |
|------|------|----------|
| computer | L26801 硬编码为 `mouse.click` + 空 params，完全不能做任何有用的事 | 小：需要实现真正的 computer use 接口或移除 |
| generate_3d / generate_sound / generate_music / generate_voice / auto_rig / generate_motion / generate_texture | 7个工具全是薄透传封装（L41522-41546），无输入验证、无 prompt 非空检查、无失败记忆、无 schema 定义（deferred 加载但无独立 schema）| 中：每个需加输入验证+失败记忆+deferred schema |
| search_game_assets / download_asset | 薄透传（L41548-41575），无输入验证、无失败记忆 | 小-中 |

### P1：有缺陷但可用（建议重构）

| 工具/工具组 | 问题 | 重写工作 |
|-------------|------|----------|
| **所有 ~60 个 deferred 搜索工具** | 无独立失败记忆（不在 `_failedSearchQ` 覆盖范围内）、无结果缓存、大部分无"何时用"描述 | 大：需统一加失败记忆+缓存+schema增强 |
| **所有 Git 工具（14个）** | 无失败记忆——git push/clone/pull 失败后模型可无限重试 | 中：需加 `_failedGitOps` 类似机制 |
| **write_file / edit_file / multi_edit** | 无写入失败记忆——写入失败后模型可反复尝试同一路径/同一 old_string | 中：需加 `_failedWriteAttempts` |
| **db_query** | 无查询失败记忆、无连接池、无慢查询警告 | 小-中 |
| **browser** | 浏览器锁机制与子智能体协同不足 | 小 |
| **delete_path / move_path / copy_path / create_dir** | 无失败记忆、无操作确认 | 小 |
| **format_file** | 无格式化失败回退 | 小 |
| **gh_pr_* / gh_actions_log** | 无失败记忆、无结果缓存 | 小-中 |

### P2：可优化（低优先级）

| 工具/工具组 | 问题 | 优化工作 |
|-------------|------|----------|
| read_terminal / list_terminals / stop_terminal | 无失败记忆 | 小 |
| lsp_symbols / lsp_definition / lsp_references | 无失败记忆、无结果缓存 | 小 |
| recall_conversation / remember | 无失败记忆 | 小 |
| screenshot / visual_compare | 无失败记忆 | 小 |
| local_discovery / live_environment / live_markets | 无失败记忆 | 小 |
| track_shipment / shop_catalog | 功能有限 | 中 |
| realtime_news_feed | 源数量有限 | 中 |
| performance_profile | 仅支持 localhost | 中 |
| openapi_parser | 无结果缓存 | 小 |
| generate_test_cases | 无失败记忆 | 小 |
| docker_compose_up | 无失败记忆 | 小 |

---

## 四、重构优先级与预计工作量

### 第一优先级（P0，预计 2-3 天）

1. **computer 工具**：要么实现真正的 computer use 接口，要么从 schema 中移除
2. **7 个 3D/音频生成工具**：加输入验证（prompt 非空检查）、失败记忆、deferred schema 定义

### 第二优先级（P1-核心，预计 5-7 天）

3. **统一 deferred 搜索工具增强**：
   - 加 `_failedSearchQ` 覆盖所有 `_search` 后缀工具（当前只覆盖 search/websearch）
   - 加结果缓存（TTL 5分钟的 `_searchResultCache`）
   - 增强 schema description（每个工具加"何时用"+vs 替代工具）
4. **Git 工具失败记忆**：加 `_failedGitOps` Map，push/clone/pull 失败后递增门控
5. **写操作工具失败记忆**：加 `_failedWriteAttempts` Map，write/edit 失败后递增门控

### 第三优先级（P1-补充，预计 3-4 天）

6. **db_query 增强**：查询失败记忆、慢查询警告、连接池管理
7. **browser 子智能体协同**：允许子智能体在隔离上下文中使用浏览器
8. **gh_pr_* 工具**：加失败记忆和结果缓存

### 第四优先级（P2，预计 2-3 天）

9. **LSP 工具**：加结果缓存
10. **文件操作辅助工具**（delete/move/copy/create_dir/format）：加失败记忆
11. **地理/生活工具**：加失败记忆
12. **新工具完善**：generate_test_cases/docker_compose_up/performance_profile/openapi_parser/realtime_news_feed 加失败记忆

**总预计工作量：12-17 天**

---

## 五、与智能体基座协同的统一改造方案

### 5.1 失败记忆统一框架

当前只有 read/find 类工具有 `_failedPathAttempts`，search/websearch 有 `_failedSearchQ`，cmd 有 `_repeatedFailedCmdShortCircuit`。需要统一为：

```
run._toolFailureMemory = new Map()
// key: `${toolType}:${normalizedArg}`
// value: { count, lastFailReason, lastFailTime, blocked }
```

所有工具执行失败时递增计数，3次以上物理拦截。

### 5.2 结果缓存统一框架

当前只有 web_fetch/web_search 有 `_agentWebCache`。需要统一为：

```
run._toolResultCache = new Map()
// key: `${toolType}:${normalizedArg}`
// value: { result, timestamp, ttl }
```

只读工具（search/lsp/git status/log/blame等）默认 TTL 5分钟。

### 5.3 Tool Ledger 分类增强

当前 `_toolLedger` 记录 tool/args/ok/reason。建议增加：
- `category`：工具能力域分类
- `failCategory`：失败分类（已有 `_classifyToolFailure` 但未全面使用）
- `retryCount`：该工具被重试的次数

### 5.4 Schema Description 标准化

每个工具的 schema description 应包含：
1. **功能描述**：一句话说明做什么
2. **何时用**：具体触发场景
3. **vs 替代**：与相似工具的区别
4. **何时不用**：不该用的场景

当前只有 web_search、search、find_symbol、semantic_search 等少数工具有"何时用"描述。

---

## 六、关键行号索引

| 功能 | 行号 |
|------|------|
| `_buildAgentToolSchemas` | L25331 |
| `_mapToolCall` | L26541 |
| `_executeToolStep` | L40361 |
| `_agentFindFiles`（重写后） | L30975-31023 |
| `_agentDirEntryName` / `_agentDirEntryIsDir` | L18878-18882 |
| `_failedPathAttempts` 渐进门控 | L40464-40482 |
| `_repeatedFailedCmdShortCircuit` | L28615 |
| `_failedSearchQ` 搜索去重 | L37145-37260 |
| `_toolLedger` 记录 | L35825-35853 |
| `_toolExpRecord` 跨会话经验 | L35192 |
| `_contentSignature` 内容签名 | L40680 |
| `_dupReadN` 冗余读拦截 | L40693 |
| deferred 搜索统一执行 | L42965-42988 |
| `_tauriSearchInvokeArgs` | L31951 |
| `_dbQueryWithRetry` | L44644-44659 |
| `_STRICT_MUTATING_TOOL_NAMES` | L26170-26178 |
| `_fileToolArgIssue` 写入参数校验 | L26135-26168 |
| `_TOOL_ALIASES` 工具别名 | L25897-25982 |
| `_canonicalToolName` 名称规范化 | L25994-26008 |
| `_normalizeArgKeys` 参数名别名 | L26009-26037 |
| `_coerceSchemaTypes` 类型自愈 | L26300-26319 |
| `TOOL_METADATA` | tool-guides.js L3-247 |
| `TOOL_EXAMPLES` | tool-guides.js L252-366 |
