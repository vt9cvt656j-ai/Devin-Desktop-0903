# 工具协调实用手册

**目标**: 从工具全景、决策地图、组合链、反例、落地建议五个维度，帮助 AI 智能体在正确的时刻用正确的工具组合。

**更新日期**: 2026年7月30日  
**数据源**: src/main.js (278工具schema) + src/tool-guides.js (TOOL_METADATA)

---

## 第一部分：工具全景（按能力域分类）

### 工具 278 个，分为 18 大能力域

#### 1. 文件系统操作（11 个工具）
核心工具: **read_file** (必用) | **list_dir** (探索) | **write_file** (新建) | **edit_file** (改)
其他: find_files, delete_path, move_path, create_dir, copy_path, format_file, search

**使用纪律**: read_file一次性读完整（别分段猜），edit_file精确定位修改，write_file只用于新建或完全重写

---

#### 2. 代码搜索与定位（7 个工具）
核心工具: **find_symbol** (查符号定义) | **lsp_references** (查调用方) | **semantic_search** (按功能找代码)
其他: search, lsp_symbols, lsp_definition, knowledge_search

**快速判断**: 只知道名字→find_symbol；有位置→lsp_definition；要找用法→lsp_references；不知道关键词→semantic_search

---

#### 3. 版本控制（14 个工具）
核心工具: **git_commit** (保存改动) | **git_log** (查历史) | **git_blame** (查行来源) | **git_branch** (管分支)
其他: git_status, git_diff, git_push, git_pull, git_clone, git_stash, git_conflicts, worktree

**核心操作**: status→diff→log/blame(理解)→edit→commit→push

---

#### 4. GitHub/CI 工具（6 个工具）
用途: PR创建/查看、CI日志、code review评论
工具: gh_pr_create, gh_pr_view, gh_pr_checks, gh_actions_log, gh_pr_review_comments, gh_pr_reply

---

#### 5. 终端与进程（5 个工具）
核心: **run_in_terminal** (dev server等长任务) | **run_cmd** (npm test等短命令)
其他: read_terminal, list_terminals, stop_terminal, read_logs

**约束**: 长任务→run_in_terminal+read_terminal；短任务→run_cmd

---

#### 6. 诊断与性能（2 个工具）
工具: **get_diagnostics** (编辑器错误) | **performance_profile** (前端性能分析)

---

#### 7. 浏览器自动化（6 个工具）
核心: **browser** (交互式) | **screenshot** (渲染截图)
其他: visual_compare, read_screen, ui_click, system

**用途**: 登录/表单/多步交互→browser；看视觉效果→screenshot；对比设计→visual_compare

---

#### 8. 网络与HTTP（4 个工具）
核心: **http_request** (调API) | **web_search** (通用搜索)
其他: web_fetch, tor_request

**纪律**: 有专用工具就用专用（developer_community_search/package_search/github_search等），web_search仅为最后手段

---

#### 9-15. 搜索工具集（>60 个工具）
- **代码与开源**: github_search, github_repo, github_trending, gitlab_repo, gitee_repo, codeberg_repo, gitlab_search, gitee_search
- **技术社区**: developer_community_search, stackoverflow_search, hackernews_search, reddit_search, v2ex_search, segmentfault_search
- **设计与UI**: awwwards_search, dribbble_search, codrops_search, smashingmag_search, css_tricks_search
- **包与依赖**: package_search, maven_search, rubygems_search, nuget_search, bundlephobia_search, cdnjs_search
- **学术**: academic_search, arxiv_search, openalex_search, pubmed_search, crossref_search, pubchem_search, clinical_trials_search
- **其他**: mdn_search, wiki_search, cve_search, docker_search, deep_search, realtime_news_feed等

**快速地图**: npm包→package_search；技术选型→developer_community_search；漏洞→cve_search；学术论文→arxiv_search

---

#### 16. 数据库操作（2 个工具）
工具: **db_query** (直连查询) | **backup_database** (备份)

**约束**: 需要真实表结构/数据前必须db_query，不要猜schema

---

#### 17. 设计与UI系统（7 个工具）
核心: **learn_design** (学标杆) | **figma** (读设计文件) | **design_board** (设计选择器)
其他: design_research, visual_explain, preview_choices, generate_image

**触发条件**: 做UI/网站前必须learn_design提取设计系统，避免凭记忆编配色

---

#### 18. 智能体编排（3 个工具）
核心: **run_worker** (派能改文件的worker) | **run_subagent** (派只读专家) | **await_subagent** (等待汇合)

**约束**: 单文件调查不派subagent，需要系统性审查或后台并行才派；默认异步(wait=false)，需要结果时await_subagent

---

## 第二部分：场景决策地图（可执行的工作流）

### 场景 A: 查符号定义与调用方
```
find_symbol("funcName") → lsp_references(path, line) → read_file() → 评估改动范围
```

### 场景 B: 初探陌生代码库
```
list_dir("src") → semantic_search("核心功能") → read_file(入口) → generate_wiki()
```

### 场景 C: 技术选型
```
package_search("库名") → developer_community_search("vs对比") → github_search("用户项目") 
→ web_search("官方文档") → debate(多角度) → 决策
```

### 场景 D: 性能卡顿定位
```
performance_profile(url) → screenshot(逐帧) → capture_start() → browser(...) 
→ capture_flows() → semantic_search("优化方法") → knowledge_search("最佳实践")
```

### 场景 E: Bug根因调查
```
run_in_terminal(dev_server) → capture_start() → browser(复现) → read_logs() 
→ db_query(验证数据) → semantic_search() → git_log(改动历史) 
→ (可选后台) run_subagent() → edit_file(修复) → run_cmd(test) → browser(验证) → git_commit()
```

### 场景 F: 建站/UI设计
```
learn_design(标杆URL) → design_research() → (generate_image多方向) 
→ design_board(用户选择) → figma(提取配色token) → write_file(实现) 
→ visual_compare(对比) → deploy_site(上线)
```

### 场景 G: 大项目多模块并行
```
openapi_parser() + db_query() [确认契约]
→ run_worker(后端) + run_worker(前端) + run_worker(数据库) [并行派发]
→ run_subagent(architect, 后台) [架构审查不阻塞]
→ await_subagent(all) [汇合]
→ run_in_terminal(dev) + run_cmd(test_e2e) [集成验证]
```

### 场景 H: 调研需求与简化
```
generate_wiki() → semantic_search(设计决策) → run_subagent(product, 后台) 
→ web_search(竞品) → awwwards_search(标杆设计) 
→ preview_choices(多方案) → await_subagent() [汇合子智能体]
```

---

## 第三部分：组合纪律（8 条核心原则）

### 纪律 1: 先取证再改
改代码前必须有真实证据（文件内容/报错/测试失败）
```
read_file() → run_cmd(test) → get_diagnostics() → git_blame() → 然后才 edit_file()
```

### 纪律 2: 改完验证闭环必须closed
```
edit_file() → run_cmd(test) → get_diagnostics() → browser(verify) → git_commit()
验证缺一项都不能提交
```

### 纪律 3: 失败先诊断再重试
```
run_cmd(...失败) → read_logs() → 根因诊断 → 修复根因 → 重试
不要盲目retry或删node_modules
```

### 纪律 4: 不确定范围就全搜一遍
```
删变量前: find_symbol() + lsp_references() 确认无引用
改API前: git_log() + lsp_references() 查所有调用方
```

### 纪律 5: 单文件调查别派子智能体
```
主智能体直接: find_symbol() → read_file() → lsp_references() 更快
别派subagent做单文件读取（过度设计）
```

### 纪律 6: 后台任务用异步不要主线等
```
run_subagent(..., wait: false)  [默认异步，立刻返回job号]
await_subagent(job: "all")  [需要时显式汇合]
主线继续做其他事，不卡住
```

### 纪律 7: 专用工具优先，web_search最后
```
npm包 → package_search (不是web_search)
技术选型 → developer_community_search (不是web_search)
学术论文 → arxiv_search (不是academic_search)
```

### 纪律 8: 弱模型/cost敏感时收敛工具
```
优先Tier1: {read_file, search, find_symbol, lsp_references, git_*, run_cmd}
少用Tier4: {各类XXX_search, 非关键路径工具}
```

---

## 第四部分：反例与禁忌

### 反例 1: 单文件调查派子智能体 ❌
```
错误: run_subagent(description: "查bug", prompt: "找src/auth.js里的问题")
正确: find_symbol() → read_file("src/auth.js") → lsp_references()
```

### 反例 2: 纯后端不触发michael-design ❌
```
错误: 写npm package用learn_design设计API格式
正确: 用openapi_parser或知识库查API设计规范
```

### 反例 3: 重复读同一文件 ❌
```
错误: read_file(limit:100) → ... → read_file(limit:200) → ... → read_file()
正确: 一次 read_file() 读完整，不传limit
```

### 反例 4: 盲目改代码不看日志 ❌
```
错误: run_cmd失败 → delete_path(node_modules) → npm install
正确: run_cmd失败 → read_logs() → 诊断根因 → 精准修复
```

### 反例 5: 用浏览器抄数据 ❌
```
错误: browser() 一页页截图JSON数据
正确: http_request() 或 web_fetch() 直接取
```

### 反例 6: 不澄清需求就改 ❌
```
错误: 用户说"登录有问题" → 直接run_in_terminal试
正确: ask_user(options: ["登不上", "登上登不出", "报错", "太慢"])
```

### 反例 7: 纯需求改动启两个dev server ❌
```
错误: 改后端API + 启npm run dev (前端) + npm run server (后端)
正确: 改后端仅 run_in_terminal(npm run server) + http_request测API
```

### 反例 8: 不确定就乱删代码 ❌
```
错误: "这个变量看起来没用" → edit_file删除
正确: find_symbol() + lsp_references() 确认真无引用后再删
```

---

## 第五部分：落地建议

### 建议 1: 沉淀场景地图进编排器
**执行**: 在_buildToolHint或orchestrator中加入场景关键字→工具集映射
```javascript
const SCENARIO_TOOLS = {
  'bug_diagnosis': [find_symbol, read_file, lsp_references, read_logs, db_query, git_blame, git_log],
  'feature_design': [learn_design, figma, design_board, write_file, visual_compare],
  'tech_research': [package_search, developer_community_search, github_search, web_search]
};
```

### 建议 2: 给出工具使用频率建议
**目标**: 让模型主动调用高ROI工具
```javascript
// 按ROI排序工具建议，不强制但引导
const TOOL_PRIORITY_BY_SCENARIO = {
  code_review: [find_symbol, lsp_references, semantic_search, git_blame, git_log],
  debugging: [run_cmd, read_logs, get_diagnostics, db_query, browser],
  ui_building: [learn_design, figma, design_board, screenshot, browser]
};
```

### 建议 3: 反例融入编排器提示词
**做法**: 在工具选择前加约束条件
```
if (task.contains("单文件") && wants_to_use(run_subagent)) {
  warn("不必派子智能体，主智能体直接read_file更快");
}
if (wants_to_use(web_search) && has_specialized_tool_available(package_search)) {
  suggest(package_search_instead);
}
```

### 建议 4: 打造"工具协调巡检清单"
**下游应用**: code review时问：
- [ ] 改代码前看了源文件吗？(read_file)
- [ ] 改代码前跑测试确认了吗？(run_cmd)
- [ ] 查了所有调用方吗？(lsp_references)
- [ ] 改完验证了吗？(run_cmd/browser验证)
- [ ] 提交信息清晰吗？(git_commit with good message)

### 建议 5: 建立"工具组合SOP"（推荐不强制）
**目标**: 为常见场景固化最优工具链，但保留灵活性
```
SOP: 修复前端bug
1. read_logs / read_terminal [采集证据]
2. browser(fresh=true) + capture_flows [复现并对比请求]
3. get_diagnostics [编辑器错误清单]
4. semantic_search / lsp_references [定位代码]
5. edit_file [修复]
6. run_cmd(test) [单测]
7. browser验证 [E2E]
8. git_commit [提交]
```

### 建议 6: 标记"禁忌工具组合"
**实现**: 在编排器中加红旗检测
```
RED_FLAG_COMBOS = [
  (run_subagent, "for single file inspection"),
  (learn_design, "for CLI/backend tools"),
  (browser, "for data scraping instead of http_request"),
  (repeated read_file, "of same file multiple times")
];
```

### 建议 7: 实时反馈工具效率
**UX改进**: 用户看到操作时显示"预期耗时"
```
run_cmd(command) → 显示 "~2s, 返回exit code"
semantic_search() → 显示 "~3s, 找代码块"
browser() → 显示 "~8s, 含截图"
read_file() → 显示 "~0.5s"
```

### 建议 8: 建档案库"工具组合失败案例"
**目的**: 持续优化决策地图，收集真实使用反馈
```
FAILURE_CASES = [
  {
    mistake: "为单文件调查派了run_subagent",
    cost: "多等3秒，通讯开销",
    fix: "检测到单文件时拦截，用主智能体read"
  },
  {
    mistake: "盲目retry run_cmd 3次再read_logs",
    cost: "浪费6秒，本可1秒诊断",
    fix: "run_cmd失败自动read_logs，不要retry"
  }
];
```

---

## 附录：工具全景总表

| 能力域 | 工具数 | 核心工具 | 触发条件 |
|------|------|--------|--------|
| 文件系统 | 11 | read_file, write_file, edit_file | 任何需要读写代码的时刻 |
| 代码搜索 | 7 | find_symbol, lsp_references, semantic_search | 找代码、确认引用范围 |
| 版本控制 | 14 | git_commit, git_log, git_blame | 提交、查历史、确认改动来源 |
| GitHub/CI | 6 | gh_pr_create, gh_actions_log | 提PR、查CI失败 |
| 终端 | 5 | run_in_terminal, run_cmd, read_logs | 启动服务、跑测试、看日志 |
| 诊断 | 2 | get_diagnostics, performance_profile | 看编辑器错误、定位卡顿 |
| 浏览器 | 6 | browser, screenshot, visual_compare | 交互测试、渲染验证、设计对比 |
| 网络 | 4 | http_request, web_search, web_fetch | 调API、上网搜索 |
| 搜索工具 | 60+ | (见文本细分) | 技术调研、社区经验、最佳实践 |
| 数据库 | 2 | db_query, backup_database | 验证表结构、备份 |
| 设计系统 | 7 | learn_design, figma, design_board | 建UI网站前必学标杆 |
| 智能体编排 | 3 | run_worker, run_subagent, await_subagent | 派发多worker并行、后台深度调研 |
| 其他 | 10+ | deploy_site, docker_compose_up, generate_image等 | 上线、启容器、生成素材 |

---

**总结**: 
- **工具总数**: 278 个
- **关键纪律**: 先取证→再改→验证闭环，禁用盲目重试
- **快速路径**: 代码问题用{find_symbol, lsp_references, git_blame}；性能问题用{performance_profile, capture_flows}；设计用{learn_design, figma, design_board}
- **反模式**: 别派子智能体做单文件读、别用browser抄数据、别重复读同一文件

本手册是参考指南，不是绝对规定。不同任务可灵活调整，但核心纪律（先取证、验证闭环、诊断根因）不动摇。

