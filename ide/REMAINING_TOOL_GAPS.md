# Michael IDE 剩余工具漏洞扫描报告

> 扫描时间：2026-07-31 | 扫描范围：162+ 工具 | 只读调研，未改代码

---

## 总体评估

**整体完成度：约 85%**。核心工具（read_file/web_search/git 等）已达到"正常人类逻辑"标准；统一失败记忆框架（#85）和结果缓存框架（#88）已覆盖大部分工具。剩余漏洞主要集中在：Rust 后端原始错误泄漏、部分工具未接入统一框架、少量描述缺失。

---

## 一、按维度扫描结果

### 1. 错误处理（友好性）

| 状态 | 工具 | 问题 | 评分 |
|------|------|------|------|
| ✅ 已修 | read_file | `friendly_read_error()` 按 ErrorKind 返回中文提示 | 5/5 |
| ✅ 已修 | net.rs (http_request/web_fetch) | URL 验证 + SSRF 防护 + 中文错误 | 5/5 |
| ✅ 已修 | db_query | driver/url/query 空值检查 + 友好错误 | 5/5 |
| ✅ 已修 | game asset tools | 失败计数 + 配置检查 + 工作区检查 | 4/5 |
| ✅ 已修 | location tools | kind 枚举验证 + 坐标验证 + 定位失败呈现 | 4/5 |
| ⚠️ 待修 | create_dir | `.map_err(\|e\| e.to_string())` 泄漏原始 OS error | 2/5 |
| ⚠️ 待修 | rename_path | 同上，OS error 直出 | 2/5 |
| ⚠️ 待修 | copy_path | 同上 | 2/5 |
| ⚠️ 待修 | delete_path | 同上 | 2/5 |
| ⚠️ 待修 | write_tmp_file | `.map_err(\|e\| e.to_string())` | 3/5 |

**文件**：`src-tauri/src/files.rs` 行 1496, 1512, 1550-1554, 1566-1568, 1467

**结论**：文件操作写入类工具（create_dir/rename_path/copy_path/delete_path）仍泄漏原始 OS error，与 read_file 已有的 `friendly_read_error()` 模式不一致。

### 2. 失败记忆（统一框架接入）

| 状态 | 工具类别 | 说明 |
|------|----------|------|
| ✅ 已接入 | 所有核心工具 | `_executeToolStep` 统一框架（main.js:44934-44957） |
| ✅ 已接入 | git/gh/db | 细粒度键 `git:push`/`gh:create` 分开计数 |
| ✅ 已接入 | 60+ deferred 搜索 | 全部在 `_CACHEABLE_TOOL_TYPES` 中 |
| ✅ 已接入 | game asset tools | 有独立失败计数但也被统一框架覆盖 |
| ⚠️ 未接入 | liveenvironment/livemarkets/liveflights/roadenvironment | catch 块只返回 `[失败]`，未调用 `_recordToolFailure` |
| ⚠️ 未接入 | track_shipment | 同上 |
| ⚠️ 未接入 | learndesign / design_research | 无失败记录 |
| ⚠️ 未接入 | debate | 有配置检查但辩论失败未记录 |
| ⚠️ 未接入 | realtime_news_feed | 部分源失败时未记录 |

**结论**：location 类工具和 design 类工具的失败未被统一失败记忆框架追踪，连续失败不会自动拦截。

### 3. 结果缓存

| 状态 | 工具 | 说明 |
|------|------|------|
| ✅ 已缓存 | read/list/search/findfiles | 核心只读工具，5min TTL |
| ✅ 已缓存 | 60+ deferred 搜索 | 全部纳入缓存 |
| ✅ 已缓存 | git status/diff/log/blame | 只读 git 操作 |
| ✅ 已缓存 | LSP 工具 | symbols/definition/references |
| ⚠️ 未缓存 | performance_profile | 只读分析，相同 URL 可缓存 |
| ⚠️ 未缓存 | openapi_parser | 只读解析，相同文件可缓存 |
| ⚠️ 未缓存 | generate_test_cases | 相同文件+框架可缓存 |
| N/A | docker_compose_up | 执行类工具，不适合缓存 |
| N/A | debate | 非确定性输出，不适合缓存 |

**结论**：遗漏的缓存目标都是只读分析类工具，影响较小（调用频率低）。

### 4. 输入验证

| 状态 | 工具 | 问题 |
|------|------|------|
| ✅ 已修 | read_file | 空 path 检查 |
| ✅ 已修 | db_query | 空 url/query 检查 |
| ✅ 已修 | generate_3d 等 | `_error` 字段 + 工作区检查 |
| ✅ 已修 | docker_compose_up | 路径正则 + 服务名白名单 |
| ✅ 已修 | performance_profile | localhost URL 限制 |
| ✅ 已修 | openapi_parser | URL 非空检查 |
| ✅ 已修 | realtime_news_feed | topic 非空检查 |
| ⚠️ 缺失 | academic_search (Rust) | query 空值未检查就发 HTTP 请求 |
| ⚠️ 缺失 | cve_search (Rust) | 同上 |
| ⚠️ 缺失 | steam_search (Rust) | 同上 |
| ⚠️ 缺失 | 所有 deferred search (Rust) | 60+ 工具均未在 Rust 层验证 query 非空 |
| ⚠️ 缺失 | format_file | 未检查 path 非空 |
| ⚠️ 缺失 | multi_edit | 未检查 edits 数组非空 |

**结论**：Rust 层 60+ deferred 搜索工具缺少 query 空值验证（虽然前端通常不会传空值，但防御性编程缺失）。

### 5. 描述缺失（usage_note）

| 状态 | 工具 | 问题 |
|------|------|------|
| ✅ 已修 | 核心工具 | 全部有【何时用】【vs 替代】【何时不用】 |
| ✅ 已修 | 60+ deferred 搜索 | P1 重构已批量补 usage_note |
| ✅ 已修 | git 工具 | 全部有 usage_note |
| ✅ 已修 | game asset 工具 | 全部有 usage_note |
| ⚠️ 缺失 | profiler | TOOL_METADATA 中无 usage_note |
| ⚠️ 缺失 | backup_database (行 191) | 有 usage_note 但在重复定义处(行 519) |
| ⚠️ 缺失 | db_migrate (行 184) | 同上，行 518 有 |
| ⚠️ 缺失 | browser_launch (行 168) | 无 usage_note（"browser" 行 537 有） |
| ⚠️ 重复 | codeberg_search | 行 455 和 496 重复定义 |
| ⚠️ 重复 | developer_community_search | 行 13 和 447 重复定义 |
| ⚠️ 重复 | db_query | 行 158 和 517 重复定义 |
| ⚠️ 重复 | realtime_news_feed | 行 321 和 500 重复定义 |

---

## 二、按能力域分类评分

### 文件操作（read_file/write_file/edit_file 等）

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| read_file | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| write_file | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| edit_file | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| multi_edit | 3/5 | ✅ | ✅ | N/A | ⚠️ | ✅ |
| delete_path | 3/5 | ⚠️ OS error | ✅ | N/A | ✅ | ✅ |
| move_path (rename_path) | 3/5 | ⚠️ OS error | ✅ | N/A | ✅ | ✅ |
| copy_path | 3/5 | ⚠️ OS error | ✅ | N/A | ✅ | ✅ |
| create_dir | 3/5 | ⚠️ OS error | ✅ | N/A | ✅ | ✅ |
| format_file | 3/5 | ✅ | ✅ | N/A | ⚠️ | ✅ |

### 搜索定位（search/find_files/lsp_symbols 等）

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| search | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| find_files | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| lsp_symbols | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| find_symbol | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| semantic_search | 4/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| knowledge_search | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |

### 执行验证（run_cmd/run_in_terminal 等）

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| run_cmd | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| run_in_terminal | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| read_logs | 4/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| read_terminal | 4/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| get_diagnostics | 4/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| background_monitor | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| stop_terminal | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |

### Git 工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| git_status | 5/5 | ✅ | ✅ 细粒度 | ✅ | ✅ | ✅ |
| git_diff | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| git_log | 4/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| git_blame | 4/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| git_commit | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| git_branch | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| git_push | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| git_pull | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| git_stash | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| gh_pr_create | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |

### 浏览器工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| browser | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| screenshot | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| visual_compare | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |

### 网络工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| http_request | 5/5 | ✅ | ✅ | ✅(GET) | ✅ | ✅ |
| web_search | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| web_fetch | 5/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| download_file | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| tor_request | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |

### 数据库工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| db_query | 5/5 | ✅ | ✅ 细粒度 | N/A | ✅ | ✅ |

### 知识检索（60+ deferred 搜索工具）

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| academic_search | 3/5 | ✅ | ✅ | ✅ | ⚠️ Rust 层无空值检查 | ✅ |
| cve_search | 3/5 | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| github_search | 3/5 | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| package_search | 3/5 | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| ... (其余 57 个同类) | 3/5 | 同上 | ✅ | ✅ | ⚠️ | ✅ |

**共性问题**：Rust 层 `query: String` 参数未验证空值即发 HTTP 请求。

### 设计研究工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| design_research | 3/5 | ✅ | ⚠️ 未接入 | N/A | ✅ | ✅ |
| learn_design | 3/5 | ✅ | ⚠️ 未接入 | N/A | ✅ | ✅ |
| design_board | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |
| visual_compare | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |

### 系统级工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| system | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |
| automation | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |
| read_screen | 3/5 | ✅ | ✅ | ✅ | ✅ | ✅ |
| ui_click | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |
| decode_qr | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |

### 实时数据工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| live_markets | 3/5 | ✅ | ⚠️ 未接入 | N/A | ✅ | ✅ |
| live_flights | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |
| live_environment | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |
| road_environment | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |

### 物流电商工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| track_shipment | 3/5 | ✅ | ⚠️ 未接入 | N/A | ✅ | ✅ |
| shop_catalog | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |
| local_discovery | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |

### 智能体协作工具

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| run_subagent | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| run_worker | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| debate | 3/5 | ✅ | ⚠️ 未接入 | N/A | ✅ | ✅ |
| worktree | 3/5 | ✅ | ⚠️ | N/A | ✅ | ✅ |

### 新工具（#10/#11）

| 工具 | 评分 | 错误处理 | 失败记忆 | 缓存 | 验证 | 描述 |
|------|------|----------|----------|------|------|------|
| generate_test_cases | 4/5 | ✅ | ✅ | ⚠️ 可缓存 | ✅ | ✅ |
| docker_compose_up | 4/5 | ✅ | ✅ | N/A | ✅ | ✅ |
| performance_profile | 4/5 | ✅ | ✅ | ⚠️ 可缓存 | ✅ | ✅ |
| openapi_parser | 4/5 | ✅ | ✅ | ⚠️ 可缓存 | ✅ | ✅ |
| realtime_news_feed | 4/5 | ✅ | ⚠️ 部分源失败未记录 | ⚠️ | ✅ | ✅ |

---

## 三、优先级排序

### P0 — 必须修（影响用户体验/安全）

| # | 问题 | 影响范围 | 修复建议 |
|---|------|----------|----------|
| P0-1 | 文件操作工具泄漏原始 OS error | create_dir/rename_path/copy_path/delete_path (4 个工具) | 仿照 `friendly_read_error()` 模式，在 files.rs 添加 `friendly_write_error()` |
| P0-2 | Rust 层 60+ 搜索工具无 query 空值验证 | knowledge.rs 中所有 `*_search` 函数 | 在每个函数开头加 `if query.trim().is_empty() { return Err("缺少查询关键词") }` |

### P1 — 建议修（影响一致性/健壮性）

| # | 问题 | 影响范围 | 修复建议 |
|---|------|----------|----------|
| P1-1 | location 工具未接入统一失败记忆 | liveenvironment/livemarkets/liveflights/roadenvironment/trackshipment (5 个) | 在 main.js catch 块加 `_recordToolFailure(call.type)` |
| P1-2 | design 工具未接入失败记忆 | learndesign/design_research/visual_compare (3 个) | 同上 |
| P1-3 | 桌面自动化工具未接入失败记忆 | system/automation/ui_click/decode_qr (4 个) | 同上 |
| P1-4 | tor_request 未接入失败记忆 | tor_request (1 个) | 同上 |
| P1-5 | realtime_news_feed 部分源失败未记录 | realtime_news_feed | 源全部失败时 `_recordToolFailure` |
| P1-6 | debate 失败未记录 | debate | 配置正确但辩论执行失败时记录 |

### P2 — 可优化（代码质量/维护性）

| # | 问题 | 影响范围 | 修复建议 |
|---|------|----------|----------|
| P2-1 | 只读分析工具未缓存 | performance_profile/openapi_parser/generate_test_cases | 加入 `_CACHEABLE_TOOL_TYPES` |
| P2-2 | tool-guides.js 重复定义 | codeberg_search/developer_community_search/db_query/realtime_news_feed | 删除重复条目 |
| P2-3 | profiler 缺 usage_note | profiler (1 个) | 补充 `usage_note: '【何时用】分析后端/API 性能瓶颈...'` |
| P2-4 | browser_launch 缺 usage_note | browser_launch (行 168) | 补充或合并到 browser 条目 |
| P2-5 | multi_edit 缺 edits 非空验证 | multi_edit | 前端加 `if (!edits.length)` 检查 |
| P2-6 | format_file 缺 path 非空验证 | format_file | 前端加 `if (!path)` 检查 |

---

## 四、修复策略建议

### 批量处理（推荐）

1. **P0-2 批量修复**：knowledge.rs 中 60+ 搜索函数，统一在 `kclient()?;` 后加 query 空值检查。可用宏或辅助函数统一处理。
   
2. **P0-1 批量修复**：files.rs 中 4 个写入工具，统一用 `friendly_write_error()` 替换 `.map_err(|e| e.to_string())`。

3. **P1-1~P1-6 批量修复**：在 main.js 中所有 location/design/desktop/tor 工具的 catch 块统一加 `_recordToolFailure`。模式一致，可一次性修改。

### 逐个修

- P2 级别问题影响较小，可在日常迭代中逐步修复。

---

## 五、关键文件索引

| 文件 | 职责 |
|------|------|
| `src-tauri/src/files.rs` | 文件操作 Rust 实现（P0-1 修复点） |
| `src-tauri/src/knowledge.rs` | 60+ deferred 搜索工具（P0-2 修复点） |
| `src/main.js:31240-31310` | 失败记忆 + 缓存框架定义 |
| `src/main.js:44918-44978` | `_executeToolStep` 统一框架 |
| `src/main.js:42303-42388` | location 工具执行（P1-1 修复点） |
| `src/tool-guides.js` | 工具元数据/描述（P2-2~P2-4 修复点） |

---

## 六、总结

| 维度 | 完成度 | 剩余工作量 |
|------|--------|------------|
| 错误处理 | 90% | 4 个文件操作工具 + 60+ 搜索工具空值检查 |
| 失败记忆 | 85% | ~15 个工具未接入统一框架 |
| 结果缓存 | 95% | 3 个只读分析工具可加入 |
| 输入验证 | 85% | 60+ Rust 层搜索函数缺空值检查 |
| 描述齐全 | 95% | 2-3 个工具缺 usage_note + 4 个重复定义 |

**结论**：项目整体已达到较高标准。剩余 P0 问题（OS error 泄漏 + Rust 层空值验证）预计 1-2 小时可批量修复完毕；P1 问题（失败记忆接入）约 30 分钟；P2 可留作日常优化。
