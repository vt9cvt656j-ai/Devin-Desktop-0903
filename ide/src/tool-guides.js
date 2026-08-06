// 工具元数据映射表：为每个重要工具定义使用场景、触发条件和示例调用
// 目的：让 AI 从"被动保守"转变为"主动发现和调用工具"
const TOOL_METADATA = Object.freeze({
  // 调研类工具
  github_search: {
    category: 'research',
    use_cases: ['技术调研', '寻找最佳实践', '对比方案', '查找类似实现'],
    triggers: ['陌生代码库', '技术选型', '需要参考实现', '遇到问题先搜社区'],
    example_call: "github_search(query='react hooks best practices', search_type='repositories')",
    priority: 'high',
    usage_note: '🛡️ GitHub 先验纪律要求：遇到陌生技术栈时，优先调用此工具而非盲目实现。【vs 替代】读某个已知仓库的真实内容用 github_repo；查包版本/兼容性用 package_search；本地代码库内搜索用 search。'
  },
  web_search: {
    category: 'research',
    use_cases: ['官方文档查找', '错误信息搜索', '技术教程查询', '联网查最新信息'],
    triggers: ['需要了解外部知识', '遇到报错', '查找官方文档', '用户要求联网/上网搜索', '需要时效性/最新信息'],
    example_call: "web_search(query='Vite official migration guide')",
    priority: 'high',
    usage_note: '【何时用】需要查找最新信息、官方文档、错误解决方案时。【vs 替代】代码库内搜索用 search；知识库用 knowledge_search。【何时不用】已有代码内答案时不要联网搜索。'
  },

  // 调试诊断类工具
  search: {
    category: 'diagnostics',
    use_cases: ['错误码搜索', '日志关键词查找', '代码模式匹配'],
    triggers: ['遇到报错', '需要定位问题', '搜索错误信息'],
    example_call: "search(query='TypeError.*line 42', path='src')",
    priority: 'high',
    usage_note: '【何时用】在代码库内搜索关键词、函数名、错误信息时。【vs 替代】联网搜索用 web_search；文件发现用 find_files。【何时不用】需要外部知识时用 web_search。'
  },
  read_terminal: {
    category: 'execution',
    use_cases: ['查看 dev server 启动日志', '确认后台任务状态', '检查退出码/报错'],
    triggers: ['启动了持续任务后需要看输出', '服务是否 ready', '进程是否退出'],
    example_call: "read_terminal(name='dev-server')",
    priority: 'high',
    usage_note: '【何时用】启动 dev server/watch 等持续任务后看日志。【vs 替代】一次性命令用 run_cmd；看历史文件用 read_logs。'
  },
  list_terminals: {
    category: 'execution',
    use_cases: ['查看当前后台任务列表', '确认哪些终端在跑'],
    triggers: ['不确定有哪些后台任务', '需要查看任务状态'],
    example_call: "list_terminals()",
    priority: 'medium',
    usage_note: '【何时用】不确定有哪些后台任务在跑时。【vs 替代】已知终端名直接 read_terminal。'
  },
  stop_terminal: {
    category: 'execution',
    use_cases: ['停止不再需要的后台任务', '释放端口冲突'],
    triggers: ['后台任务不再需要', '端口冲突需要杀旧进程'],
    example_call: "stop_terminal(name='old-server')",
    priority: 'medium',
    usage_note: '【何时用】后台任务不再需要或端口冲突时。【vs 替代】只是看输出用 read_terminal。'
  },
  lsp_symbols: {
    category: 'search',
    use_cases: ['快速了解文件结构', '定位函数/类大致位置'],
    triggers: ['打开陌生文件需要概览', '找导出/函数/类'],
    example_call: "lsp_symbols(path='src/main.ts')",
    priority: 'medium',
    usage_note: '【何时用】快速了解文件结构大纲。【vs 替代】精确找符号用 find_symbol；查引用用 lsp_references。'
  },
  lsp_definition: {
    category: 'search',
    use_cases: ['跳到符号定义处', '读代码时跳进实现'],
    triggers: ['需要看符号的实现', '读代码碰到陌生符号'],
    example_call: "lsp_definition(path='src/main.ts', line=42, symbol='createSession')",
    priority: 'medium',
    usage_note: '【何时用】读代码需要跳到符号定义处。【vs 替代】不知位置先 find_symbol；查引用用 lsp_references。'
  },
  lsp_references: {
    category: 'search',
    use_cases: ['查看函数被谁调用', '评估改动影响面'],
    triggers: ['重构前需要看影响范围', '想知道谁在用这个函数'],
    example_call: "lsp_references(path='src/main.ts', line=42, symbol='createSession')",
    priority: 'medium',
    usage_note: '【何时用】看函数/变量被谁调用、评估改动影响面。【vs 替代】文本搜索用 search。'
  },
  recall_conversation: {
    category: 'interaction',
    use_cases: ['找回早期被压缩的对话原文', '恢复被遗忘的决定'],
    triggers: ['上下文摘要提到但细节不详', '早期决定/报错需要原话'],
    example_call: "recall_conversation(query='database decision')",
    priority: 'medium',
    usage_note: '【何时用】找回本会话早期被压缩掉的对话原文。【vs 替代】跨会话知识用 remember/知识库。'
  },
  remember: {
    category: 'interaction',
    use_cases: ['跨会话记住项目偏好', '沉淀通用经验'],
    triggers: ['发现值得长期记住的知识', '用户明确要求记住'],
    example_call: "remember(content='Use pnpm for this workspace')",
    priority: 'medium',
    usage_note: '【何时用】把值得跨会话记住的知识写入记忆图谱。【vs 替代】当前会话内的事用 recall_conversation。'
  },
  delete_path: {
    category: 'file_io',
    use_cases: ['清理废弃文件', '重构移除旧模块'],
    triggers: ['需要删除文件/目录', '清理旧代码'],
    example_call: "delete_path(path='dist/stale.js')",
    priority: 'medium',
    usage_note: '【何时用】清理废弃文件/目录。【vs 替代】移动/改名用 move_path。不可逆操作。'
  },
  move_path: {
    category: 'file_io',
    use_cases: ['文件/目录改名', '调整目录结构'],
    triggers: ['重构需要移动文件', '目录结构调整'],
    example_call: "move_path(from='src/old.ts', to='src/new.ts')",
    priority: 'medium',
    usage_note: '【何时用】移动或重命名文件/目录。【vs 替代】保留原件用 copy_path。'
  },
  copy_path: {
    category: 'file_io',
    use_cases: ['按模板搭脚手架', '备份文件'],
    triggers: ['需要复制文件', '备份后修改'],
    example_call: "copy_path(from='config.example.json', to='config.json')",
    priority: 'medium',
    usage_note: '【何时用】复制文件或目录。【vs 替代】移动/改名用 move_path。'
  },
  create_dir: {
    category: 'file_io',
    use_cases: ['创建空目录'],
    triggers: ['需要创建目录结构'],
    example_call: "create_dir(path='src/features/auth')",
    priority: 'low',
    usage_note: '【何时用】确实需要创建空目录。【注意】write_file 会自动创建父目录，一般不需要手动建。'
  },
  format_file: {
    category: 'code_editing',
    use_cases: ['统一代码格式', '格式化整个文件'],
    triggers: ['改完代码想统一格式', '代码风格不一致'],
    example_call: "format_file(path='src/app.ts')",
    priority: 'low',
    usage_note: '【何时用】改完代码想统一格式风格时。【vs 替代】局部调整直接 edit_file。仅支持有 LSP 的语言。'
  },

  // 开发辅助工具
  ask_user: {
    category: 'interaction',
    use_cases: ['需求不明确', '技术方案多选一', '参数未确定', '优先级排序'],
    triggers: ['信息不足', '有多选方案', '需要用户决策'],
    example_call: "ask_user(question='选择部署方式', options=['Vercel', 'Docker', 'Static'], recommended=0)",
    priority: 'critical',
    usage_note: '解决需求模糊的首选工具，优先于瞎猜！但首轮禁用：先用读取/搜索工具收集证据，之后仍不确定再提问'
  },
  update_plan: {
    category: 'planning',
    use_cases: ['任务拆解', '进度跟踪', '里程碑设置'],
    triggers: ['复杂任务', '多步骤工作', '需要记录进度'],
    example_call: "update_plan(steps=[{content: 'Implement auth', status: 'in_progress'}])",
    priority: 'medium',
    usage_note: '【何时用】多文件/跨模块/多步骤任务开工时列出完整路线并随真实证据推进状态。【vs 替代】只读诊断/看日志/恢复依赖/跑验证不要套计划。【何时不用】简单一步修改不建计划。'
  },

  // Git 相关工具
  git_branch: {
    category: 'version_control',
    use_cases: ['功能分支创建', '隔离实验代码', 'PR 准备'],
    triggers: ['开始新功能', '需要隔离改动', '准备提交代码'],
    example_call: "git_branch(action='create', name='feature/session-fix')",
    priority: 'medium',
    usage_note: '【何时用】开始新功能需要隔离代码时。【vs 替代】查看状态用 git_status；提交用 git_commit。【何时不用】小修改可以直接在 main 上提交。'
  },
  git_commit: {
    category: 'version_control',
    use_cases: ['阶段性保存', '代码归档', 'PR 前提'],
    triggers: ['完成一个小阶段', '需要保存当前状态', '准备推送'],
    example_call: "git_commit(message='Fix session expiry issue')",
    priority: 'medium',
    usage_note: '【何时用】完成一个小阶段需要保存进度时。【vs 替代】创建分支用 git_branch；推送用 git_push。【何时不用】还没完成一个完整阶段时不要急于提交。'
  },

  // 文件系统操作
  read_file: {
    category: 'file_io',
    use_cases: ['代码阅读', '配置检查', '文档理解'],
    triggers: ['需要理解代码', '查看配置文件', '阅读文档'],
    example_call: "read_file(path='src/main.js')",
    priority: 'critical',
    usage_note: '【何时用】需要读取文件内容理解代码/配置时。【vs 替代】目录结构用 list_dir；搜索内容用 search。【何时不用】只需知道目录结构时用 list_dir。'
  },
  list_dir: {
    category: 'file_io',
    use_cases: ['项目结构探索', '文件发现', '目录遍历'],
    triggers: ['陌生项目', '需要找文件', '了解目录结构'],
    example_call: "list_dir(path='src')",
    priority: 'critical',
    usage_note: '【何时用】探索项目结构、发现文件位置时。【vs 替代】读文件内容用 read_file；按模式找文件用 find_files。【何时不用】已知路径直接 read_file。'
  },

  // Web 服务与 API
  web_fetch: {
    category: 'networking',
    use_cases: ['网页内容读取', '在线文档抓取', 'API 响应查看'],
    triggers: ['需要读取网页正文', '查看在线文档内容', 'web_search 找到链接后需要读内容'],
    example_call: "web_fetch(url='https://vite.dev/guide/')",
    priority: 'high',
    usage_note: '【何时用】抓取公网网页正文——读 web_search 找到的页面、在线文档、API 参考、报错信息等。【vs 替代】搜索用 web_search；调 API 发请求用 http_request。【何时不用】不需要读网页正文时；代码库内搜索用 search。'
  },

  // 代码生成与修改
  edit_file: {
    category: 'code_editing',
    use_cases: ['小范围修改', '重构代码', '修复 bug'],
    triggers: ['已知修改位置', '替换特定代码块', '精确修改'],
    example_call: "edit_file(path='src/app.ts', old_string='const ready = false;', new_string='const ready = true;')",
    priority: 'high',
    usage_note: '【何时用】精确修改已知位置的代码时。【vs 替代】新建文件用 write_file；多处修改用 multi_edit。【何时不用】文件不存在时用 write_file。'
  },
  write_file: {
    category: 'code_editing',
    use_cases: ['创建新文件', '生成配置', '写入测试结果'],
    triggers: ['新建文件', '生成代码', '保存输出'],
    example_call: "write_file(path='src/new-feature.ts', content='export const x = 1;')",
    priority: 'high',
    usage_note: '【何时用】创建新文件或完全重写现有文件时。【vs 替代】小范围修改用 edit_file。【何时不用】只改几行时用 edit_file 更安全。'
  },
  run_cmd: {
    category: 'execution',
    use_cases: ['运行测试', '构建项目', '执行脚本'],
    triggers: ['需要执行程序', '运行命令', '验证构建'],
    example_call: "run_cmd(command='npm test')",
    priority: 'high',
    usage_note: '【何时用】运行测试、构建、安装依赖等一次性命令时。【vs 替代】长时间运行的服务用 run_in_terminal。【何时不用】需要持续运行的进程用 run_in_terminal。'
  },

  // 游戏资产生成类
  generate_3d: {
    category: 'game_asset_generation',
    use_cases: ['生成 3D 模型/网格', '游戏道具建模', '场景资产创建'],
    triggers: ['需要 3D 模型', '游戏开发需要资产', '创建 3D 对象'],
    example_call: "generate_3d(prompt='Low-poly sci-fi crate', name='sci-fi-crate')",
    priority: 'medium',
    usage_note: '【何时用】游戏/3D 项目需要模型资产时。【vs 替代】2D 图片用 generate_image。【何时不用】需要 2D 图片/纹理时不要用这个。需要外部生成服务支持，未配置时会返回错误。'
  },
  generate_sound: {
    category: 'game_asset_generation',
    use_cases: ['生成音效/声效', 'UI 反馈音', '环境音效创建'],
    triggers: ['需要音效', '游戏声音效果', '短音频片段'],
    example_call: "generate_sound(prompt='Short metallic UI confirmation', name='confirm')",
    priority: 'medium',
    usage_note: '【何时用】需要短音效（0.5-300 秒）时。【vs 替代】长音乐/背景音乐用 generate_music。【何时不用】需要背景音乐/旋律时用 generate_music。需要外部生成服务支持。'
  },
  generate_music: {
    category: 'game_asset_generation',
    use_cases: ['生成背景音乐', '游戏 BGM', '循环音乐片段'],
    triggers: ['需要背景音乐', '游戏音乐', '循环旋律'],
    example_call: "generate_music(prompt='Looping calm strategy-game theme', name='strategy-loop')",
    priority: 'medium',
    usage_note: '【何时用】需要背景音乐/旋律（1-600 秒）时。【vs 替代】短音效/点击声用 generate_sound。【何时不用】需要短促音效时用 generate_sound。需要外部生成服务支持。'
  },
  generate_voice: {
    category: 'game_asset_generation',
    use_cases: ['生成语音旁白', 'NPC 对话语音', '文本转语音'],
    triggers: ['需要语音', '文字转语音', '角色对白音频'],
    example_call: "generate_voice(text='Mission complete.', name='mission-complete')",
    priority: 'medium',
    usage_note: '【何时用】需要把文字转成语音时。【vs 替代】纯音效用 generate_sound。【何时不用】不需要说话内容时用 sound/music。需要外部生成服务支持。'
  },
  auto_rig: {
    category: 'game_asset_generation',
    use_cases: ['为 3D 模型添加骨骼', '角色绑定', '模型动画准备'],
    triggers: ['需要给模型加骨骼', '角色绑定', '准备动画'],
    example_call: "auto_rig(model_path='assets/character.glb', name='hero-rig')",
    priority: 'low',
    usage_note: '【何时用】已有 3D 模型需要添加骨骼绑定时。【vs 替代】不需要骨骼就用 generate_3d。【何时不用】模型不需要动画时。需要外部服务支持。'
  },
  generate_motion: {
    category: 'game_asset_generation',
    use_cases: ['生成动画/动作', '角色动作', '运动循环'],
    triggers: ['需要动画', '角色动作', '运动效果'],
    example_call: "generate_motion(prompt='Natural walk cycle', name='walk')",
    priority: 'medium',
    usage_note: '【何时用】需要为已绑定骨骼的角色生成动画时。【vs 替代】没有骨骼先 auto_rig；静态模型用 generate_3d。【何时不用】不需要动画时。需要外部服务支持。'
  },
  generate_texture: {
    category: 'game_asset_generation',
    use_cases: ['生成纹理贴图', '材质贴图', '表面纹理'],
    triggers: ['需要纹理', '材质贴图', '表面效果'],
    example_call: "generate_texture(prompt='Seamless worn steel', name='worn-steel')",
    priority: 'medium',
    usage_note: '【何时用】需要 3D 模型的纹理/材质贴图时。【vs 替代】2D 图片用 generate_image。【何时不用】不需要纹理贴图时。分辨率 64-8192。需要外部服务支持。'
  },
  search_game_assets: {
    category: 'game_asset_generation',
    use_cases: ['搜索免费游戏资产', '查找 CC0 模型/纹理/音效', '游戏开发资源发现'],
    triggers: ['需要游戏资产', '找免费模型/贴图/音效', '资源搜索'],
    example_call: "search_game_assets(query='CC0 low-poly spaceship')",
    priority: 'medium',
    usage_note: '【何时用】需要搜索现成的游戏资产（模型/纹理/音效/动画等）。【vs 替代】找不到合适的再用 generate_* 生成。【何时不用】已有明确生成目标时直接用 generate_*。需要外部服务支持。'
  },
  download_asset: {
    category: 'game_asset_generation',
    use_cases: ['下载游戏资产到工作区', '获取远程模型/纹理文件', '资产导入'],
    triggers: ['需要下载资产', '获取远程文件', '导入模型/贴图'],
    example_call: "download_asset(url='https://example.com/asset.glb', name='spaceship.glb')",
    priority: 'medium',
    usage_note: '【何时用】从已知 URL 下载游戏资产文件。【vs 替代】不知道去哪找先用 search_game_assets；普通文件下载用 download_file。【何时不用】非资产类文件下载用 download_file。需要有效 http/https URL。'
  },

  // ── P1 重构：deferred 搜索工具批量补 usage_note ──
  developer_community_search: {
    category: 'research', use_cases: ['开发者社区聚合搜索', '多源技术讨论'], triggers: ['需要社区经验', '查找技术讨论'],
    example_call: "developer_community_search(query='Vite monorepo migration', scope='all')",
    priority: 'high',
    usage_note: '【何时用】聚合搜索 StackOverflow/Reddit/HN 等社区讨论。【vs 替代】单一平台用对应工具；代码库内用 search。【何时不用】已有明确答案时。'
  },
  gitlab_search: { category: 'research', use_cases: ['GitLab 项目搜索'], triggers: ['需要 GitLab 代码'], example_call: "gitlab_search(query='CI pipeline config')", priority: 'medium', usage_note: '【何时用】搜索 GitLab 上的公开项目/代码。【vs 替代】GitHub 用 github_search。' },
  gitee_search: { category: 'research', use_cases: ['Gitee 项目搜索'], triggers: ['需要国内开源项目'], example_call: "gitee_search(query='Vue 组件库')", priority: 'medium', usage_note: '【何时用】搜索 Gitee 上的国内开源项目。【vs 替代】国际项目用 github_search。' },
  cve_search: { category: 'research', use_cases: ['安全漏洞查询', 'CVE 编号查找'], triggers: ['安全审计', '依赖漏洞检查'], example_call: "cve_search(query='Node.js HTTP2 vulnerability')", priority: 'high', usage_note: '【何时用】查询已知安全漏洞/CVE 编号时。【vs 替代】通用搜索用 web_search。【何时不用】非安全相关调研。' },
  wiki_search: { category: 'research', use_cases: ['维基百科知识检索'], triggers: ['需要概念解释', '查找百科信息'], example_call: "wiki_search(query='CAP theorem')", priority: 'low', usage_note: '【何时用】查找百科知识/概念解释时。【vs 替代】技术文档用 web_search。' },
  stackoverflow_search: { category: 'research', use_cases: ['StackOverflow 问答搜索'], triggers: ['遇到编程问题', '查找解决方案'], example_call: "stackoverflow_search(query='TypeScript generic constraint')", priority: 'high', usage_note: '【何时用】搜索 StackOverflow 上的编程问答。【vs 替代】多源聚合用 developer_community_search。' },
  hackernews_search: { category: 'research', use_cases: ['HackerNews 讨论搜索'], triggers: ['查找技术趋势', '社区观点'], example_call: "hackernews_search(query='Rust vs Go performance')", priority: 'medium', usage_note: '【何时用】搜索 HackerNews 上的技术讨论。【vs 替代】多源聚合用 developer_community_search。' },
  dockerhub_search: { category: 'research', use_cases: ['Docker 镜像搜索'], triggers: ['需要容器镜像', '查找官方镜像'], example_call: "dockerhub_search(query='postgres')", priority: 'medium', usage_note: '【何时用】搜索 Docker Hub 上的容器镜像。【vs 替代】通用包搜索用 package_search。' },
  pubmed_search: { category: 'research', use_cases: ['生物医学文献检索'], triggers: ['医学研究', '生物信息'], example_call: "pubmed_search(query='CRISPR gene editing')", priority: 'low', usage_note: '【何时用】检索 PubMed 生物医学文献。【vs 替代】通用学术用 openalex_search。' },
  arxiv_search: { category: 'research', use_cases: ['arXiv 预印本搜索'], triggers: ['最新研究论文', 'AI/ML 前沿'], example_call: "arxiv_search(query='diffusion model sampling')", priority: 'medium', usage_note: '【何时用】搜索 arXiv 最新研究预印本。【vs 替代】已发表论文用 crossref_search。' },
  crossref_search: { category: 'research', use_cases: ['DOI/引用元数据检索'], triggers: ['查找论文引用', 'DOI 解析'], example_call: "crossref_search(query='10.1145/reactive')", priority: 'low', usage_note: '【何时用】通过 DOI 查找论文引用元数据。【vs 替代】全文搜索用 openalex_search。' },
  openalex_search: { category: 'research', use_cases: ['开放学术元数据搜索'], triggers: ['学术数据分析', '机构/作者检索'], example_call: "openalex_search(query='machine learning')", priority: 'low', usage_note: '【何时用】搜索开放学术图谱数据。【vs 替代】论文全文用 arxiv_search。' },
  pubchem_search: { category: 'research', use_cases: ['化学物质数据库检索'], triggers: ['化学信息', '分子结构'], example_call: "pubchem_search(query='aspirin')", priority: 'low', usage_note: '【何时用】查询化学物质/分子信息。【vs 替代】生物医学用 pubmed_search。' },
  clinical_trials_search: { category: 'research', use_cases: ['临床试验数据检索'], triggers: ['药物试验', '医学实验'], example_call: "clinical_trials_search(query='mRNA vaccine phase 3')", priority: 'low', usage_note: '【何时用】检索临床试验数据。【vs 替代】文献用 pubmed_search。' },
  maven_search: { category: 'research', use_cases: ['Java/Maven 包搜索'], triggers: ['Java 依赖查找'], example_call: "maven_search(query='spring-boot-starter')", priority: 'medium', usage_note: '【何时用】搜索 Maven Central 上的 Java 包。【vs 替代】通用包搜索用 package_search。' },
  packagist_search: { category: 'research', use_cases: ['PHP/Composer 包搜索'], triggers: ['PHP 依赖查找'], example_call: "packagist_search(query='laravel/sanctum')", priority: 'medium', usage_note: '【何时用】搜索 Packagist 上的 PHP 包。【vs 替代】通用包搜索用 package_search。' },
  rubygems_search: { category: 'research', use_cases: ['Ruby Gem 包搜索'], triggers: ['Ruby 依赖查找'], example_call: "rubygems_search(query='rails-activerecord')", priority: 'medium', usage_note: '【何时用】搜索 RubyGems 上的 Gem 包。【vs 替代】通用包搜索用 package_search。' },
  nuget_search: { category: 'research', use_cases: ['.NET/NuGet 包搜索'], triggers: ['C#/.NET 依赖查找'], example_call: "nuget_search(query='EntityFrameworkCore')", priority: 'medium', usage_note: '【何时用】搜索 NuGet 上的 .NET 包。【vs 替代】通用包搜索用 package_search。' },
  homebrew_search: { category: 'research', use_cases: ['Homebrew 公式搜索'], triggers: ['macOS 软件安装'], example_call: "homebrew_search(query='node')", priority: 'medium', usage_note: '【何时用】搜索 Homebrew 公式/cask。【vs 替代】通用包搜索用 package_search。' },
  mdn_search: { category: 'research', use_cases: ['MDN Web 文档搜索'], triggers: ['Web API 查询', 'CSS/HTML 参考'], example_call: "mdn_search(query='IntersectionObserver')", priority: 'high', usage_note: '【何时用】搜索 MDN Web 文档（API/CSS/HTML）。【vs 替代】通用技术搜索用 web_search。' },
  cdnjs_search: { category: 'research', use_cases: ['CDN 前端库搜索'], triggers: ['找 CDN 链接', '前端库版本'], example_call: "cdnjs_search(query='vue')", priority: 'low', usage_note: '【何时用】搜索 cdnjs 上的前端库 CDN 链接。【vs 替代】npm 包用 package_search。' },
  bundlephobia_search: { category: 'research', use_cases: ['npm 包体积分析'], triggers: ['包大小评估', 'bundle 优化'], example_call: "bundlephobia_search(query='lodash')", priority: 'medium', usage_note: '【何时用】查看 npm 包的打包体积。【vs 替代】包信息用 package_search。' },
  devto_search: { category: 'research', use_cases: ['Dev.to 技术博客搜索'], triggers: ['查找技术文章'], example_call: "devto_search(query='React Server Components')", priority: 'low', usage_note: '【何时用】搜索 Dev.to 技术博客文章。【vs 替代】多源聚合用 developer_community_search。' },
  steam_search: { category: 'research', use_cases: ['Steam 游戏搜索'], triggers: ['游戏信息', '价格查询'], example_call: "steam_search(query='roguelike')", priority: 'low', usage_note: '【何时用】搜索 Steam 上的游戏信息。' },
  iconify_search: { category: 'research', use_cases: ['图标库搜索'], triggers: ['找图标', 'UI 图标选择'], example_call: "iconify_search(query='arrow-right')", priority: 'medium', usage_note: '【何时用】搜索 Iconify 图标库中的图标。【vs 替代】通用搜索用 web_search。' },
  juejin_search: { category: 'research', use_cases: ['掘金中文技术社区搜索'], triggers: ['中文技术文章'], example_call: "juejin_search(query='Vue3 组合式 API')", priority: 'medium', usage_note: '【何时用】搜索掘金上的中文技术文章。【vs 替代】英文社区用 developer_community_search。' },
  codrops_search: { category: 'research', use_cases: ['Codrops UI 示例搜索'], triggers: ['前端 UI 效果', 'CSS 动画'], example_call: "codrops_search(query='parallax scroll')", priority: 'low', usage_note: '【何时用】搜索 Codrops 上的前端 UI 示例/教程。' },
  smashingmag_search: { category: 'research', use_cases: ['Smashing Magazine 文章搜索'], triggers: ['Web 设计/开发文章'], example_call: "smashingmag_search(query='responsive images')", priority: 'low', usage_note: '【何时用】搜索 Smashing Magazine 的设计/开发文章。' },
  awwwards_search: { category: 'research', use_cases: ['Awwwards 获奖网站搜索'], triggers: ['网页设计参考', '创意网站'], example_call: "awwwards_search(query='portfolio minimal')", priority: 'low', usage_note: '【何时用】搜索 Awwwards 获奖网站获取设计灵感。' },
  v2ex_search: { category: 'research', use_cases: ['V2EX 社区搜索'], triggers: ['中文开发者社区'], example_call: "v2ex_search(query='远程工作')", priority: 'low', usage_note: '【何时用】搜索 V2EX 中文开发者社区讨论。' },
  segmentfault_search: { category: 'research', use_cases: ['SegmentFault 中文技术问答'], triggers: ['中文编程问题'], example_call: "segmentfault_search(query='React hooks 闭包')", priority: 'low', usage_note: '【何时用】搜索 SegmentFault 中文技术问答。【vs 替代】英文用 stackoverflow_search。' },
  github_discussions_search: { category: 'research', use_cases: ['GitHub Discussions 搜索'], triggers: ['项目讨论', '功能请求'], example_call: "github_discussions_search(query='monorepo tooling')", priority: 'medium', usage_note: '【何时用】搜索 GitHub Discussions 讨论区。【vs 替代】代码搜索用 github_search。' },
  github_trending: { category: 'research', use_cases: ['GitHub 趋势项目浏览'], triggers: ['了解热门项目', '技术趋势'], example_call: "github_trending()", priority: 'medium', usage_note: '【何时用】浏览 GitHub 当前热门项目/仓库。【vs 替代】特定搜索用 github_search。' },
  infoq_search: { category: 'research', use_cases: ['InfoQ 技术文章搜索'], triggers: ['企业级技术', '架构设计'], example_call: "infoq_search(query='microservices patterns')", priority: 'low', usage_note: '【何时用】搜索 InfoQ 企业级技术文章。' },
  codeberg_search: { category: 'research', use_cases: ['Codeberg 项目搜索', '自由/开源项目发现'], triggers: ['需要自由软件项目', '非主流开源'], example_call: "codeberg_search(query='privacy browser')", priority: 'low', usage_note: '【何时用】搜索 Codeberg 上的自由/开源项目。【vs 替代】主流项目用 github_search；读某仓库真实内容用 codeberg_repo。' },
  sourcegraph_search: { category: 'research', use_cases: ['Sourcegraph 跨仓库代码搜索'], triggers: ['大规模代码搜索', '跨项目引用'], example_call: "sourcegraph_search(query='useEffect cleanup')", priority: 'medium', usage_note: '【何时用】跨多个仓库搜索代码模式/引用。【vs 替代】单仓库用 search。' },
  realtime_news_feed: { category: 'research', use_cases: ['实时技术新闻聚合'], triggers: ['了解近期动态', '技术风向'], example_call: "realtime_news_feed(topic='AI agents', sources='all')", priority: 'high', usage_note: '【何时用】聚合多源实时技术新闻。【vs 替代】历史搜索用 web_search。' },
  git_status: { category: 'version_control', use_cases: ['查看仓库改动状态'], triggers: ['需要确认改动范围', '提交前检查'], example_call: "git_status()", priority: 'high', usage_note: '【何时用】查看当前仓库改动状态。【vs 替代】看具体差异用 git_diff。' },
  git_diff: { category: 'version_control', use_cases: ['查看文件差异'], triggers: ['审查改动', '提交前复查'], example_call: "git_diff(staged=true)", priority: 'high', usage_note: '【何时用】查看文件具体差异。【vs 替代】概览用 git_status；历史用 git_log。' },
  git_log: { category: 'version_control', use_cases: ['查看提交历史'], triggers: ['追溯改动原因', '了解项目演进'], example_call: "git_log(max_count=10)", priority: 'medium', usage_note: '【何时用】查看提交历史。【vs 替代】单文件历史用 git_blame。' },
  git_blame: { category: 'version_control', use_cases: ['逐行追溯提交作者'], triggers: ['定位谁改了这行', '理解改动上下文'], example_call: "git_blame(path='src/main.ts', line=42)", priority: 'medium', usage_note: '【何时用】追溯某行代码的提交者和时间。【vs 替代】全局历史用 git_log。' },
  git_push: { category: 'version_control', use_cases: ['推送提交到远程'], triggers: ['本地提交完成', '需要同步远程'], example_call: "git_push()", priority: 'medium', usage_note: '【何时用】将本地提交推送到远程仓库。【注意】失败后不要无限重试，先检查网络/凭据。' },
  git_pull: { category: 'version_control', use_cases: ['拉取远程更新'], triggers: ['同步远程改动', '合并前更新'], example_call: "git_pull()", priority: 'medium', usage_note: '【何时用】拉取并合并远程分支更新。【注意】有冲突时先 git_conflicts 查看。' },
  git_stash: { category: 'version_control', use_cases: ['暂存当前改动'], triggers: ['需要切换分支', '临时保存改动'], example_call: "git_stash()", priority: 'low', usage_note: '【何时用】暂存当前未提交改动以便切换上下文。【vs 替代】永久保存用 git_commit。' },
  gh_pr_create: { category: 'version_control', use_cases: ['创建 Pull Request'], triggers: ['功能完成', '需要代码审查'], example_call: "gh_pr_create(title='Fix auth bug', body='Details...')", priority: 'high', usage_note: '【何时用】创建 PR 请求代码审查。【vs 替代】查看 PR 用 gh_pr_view。' },
  gh_pr_view: { category: 'version_control', use_cases: ['查看 PR 详情'], triggers: ['审查 PR 状态', '查看 PR 信息'], example_call: "gh_pr_view(number=42)", priority: 'medium', usage_note: '【何时用】查看 PR 详细信息。【vs 替代】CI 状态用 gh_pr_checks。' },
  gh_pr_checks: { category: 'version_control', use_cases: ['查看 PR CI 状态'], triggers: ['检查 CI 是否通过', '排查构建失败'], example_call: "gh_pr_checks(number=42)", priority: 'medium', usage_note: '【何时用】查看 PR 的 CI/CD 检查结果。' },
  gh_actions_log: { category: 'version_control', use_cases: ['查看 GitHub Actions 日志'], triggers: ['排查 CI 失败', '查看构建日志'], example_call: "gh_actions_log(run_id='RUN_ID')", priority: 'medium', usage_note: '【何时用】查看 Actions 运行日志排查失败。' },
  gh_pr_review_comments: { category: 'version_control', use_cases: ['查看 PR 审查评论'], triggers: ['处理 review 反馈'], example_call: "gh_pr_review_comments(number=42)", priority: 'medium', usage_note: '【何时用】查看 PR 上的审查评论。【vs 替代】回复评论用 gh_pr_reply。' },
  gh_pr_reply: { category: 'version_control', use_cases: ['回复 PR 评论'], triggers: ['回应 review 反馈'], example_call: "gh_pr_reply(number=42, body='Fixed')", priority: 'medium', usage_note: '【何时用】回复 PR 上的评论。' },
  db_query: { category: 'data_layer', use_cases: ['执行 SQL 查询', '数据库结构检查'], triggers: ['查询数据', '检查 schema'], example_call: "db_query(driver='sqlite', query='SELECT * FROM users')", priority: 'high', usage_note: '【何时用】执行 SQL 查询/检查数据库结构。【vs 替代】表结构变更请直接执行迁移脚本（run_cmd）。【注意】查询失败后不要无限重试同一条 SQL。' },
  multi_edit: { category: 'code_editing', use_cases: ['同一文件多处修改'], triggers: ['批量重命名', '多处同步改动'], example_call: "multi_edit(path='src/app.ts', edits=[{old_string:'old', new_string:'new'}])", priority: 'high', usage_note: '【何时用】同一文件需要多处修改时。【vs 替代】单处修改用 edit_file；新建文件用 write_file。' },
  semantic_search: { category: 'search', use_cases: ['语义代码搜索', '按意图找代码'], triggers: ['不知道精确关键词', '按功能描述找代码'], example_call: "semantic_search(query='where auth tokens are validated', top_k=8)", priority: 'high', usage_note: '【何时用】按语义/意图搜索代码（不需要精确关键词）。【vs 替代】精确匹配用 search；文件发现用 find_files。' },
  knowledge_search: { category: 'search', use_cases: ['项目知识库检索', '设计规范查询'], triggers: ['查找项目规范', '检索已知知识'], example_call: "knowledge_search(query='dashboard color scheme', domain='michael-design')", priority: 'high', usage_note: '【何时用】从项目知识库中检索已知信息。【vs 替代】代码搜索用 search；联网搜索用 web_search。' },
  find_symbol: { category: 'search', use_cases: ['跨文件符号查找'], triggers: ['找函数/类定义', '追踪符号引用'], example_call: "find_symbol(name='createSession')", priority: 'high', usage_note: '【何时用】查找函数/类/变量的定义和引用。【vs 替代】文本搜索用 search；语义搜索用 semantic_search。' },
  http_request: { category: 'networking', use_cases: ['HTTP API 调用', '服务健康检查', '外部服务交互'], triggers: ['调用外部 API', '验证服务状态', '发 POST/PUT 请求', '需要自定义 headers/body'], example_call: "http_request(method='GET', url='https://api.example.com/health')", priority: 'high', usage_note: '【何时用】调用任意 HTTP API——这是你用各种网上工具/在线服务的关键能力。公网 API 不要凭感觉拼路径，先用官方文档/页面源码/抓包/用户给的精确 URL 取证；localhost/dev server/已取证 URL 可直接请求。【vs 替代】只读网页正文用 web_fetch（更简单）；搜索用 web_search。【何时不用】只是读网页正文不需要自定义请求时，用 web_fetch。' },
  download_file: { category: 'networking', use_cases: ['文件下载'], triggers: ['下载资源文件', '获取远程文件'], example_call: "download_file(url='https://example.com/file.zip', dest='./downloads/file.zip')", priority: 'medium', usage_note: '【何时用】从 URL 下载文件到本地。【vs 替代】游戏资产用 download_asset。' },
  tor_request: { category: 'networking', use_cases: ['Tor 匿名网络请求'], triggers: ['需要匿名访问', '.onion 站点'], example_call: "tor_request(method='GET', url='http://example.onion/')", priority: 'low', usage_note: '【何时用】通过 Tor 网络访问 .onion 站点或匿名请求。桌面专用。' },
  performance_profile: { category: 'diagnostics', use_cases: ['前端性能分析', '页面加载检测'], triggers: ['页面加载慢', '性能瓶颈定位'], example_call: "performance_profile(url='http://localhost:5174')", priority: 'medium', usage_note: '【何时用】分析前端页面性能（仅 localhost）。【vs 替代】后端性能用 profiler。' },
  openapi_parser: { category: 'specification_parsing', use_cases: ['OpenAPI 规范解析', 'API 端点提取'], triggers: ['查看可用 API', '接口文档解析'], example_call: "openapi_parser(url='./openapi.json', outputFormat='list')", priority: 'medium', usage_note: '【何时用】解析 OpenAPI/Swagger 规范提取端点列表。' },
  docker_compose_up: { category: 'execution', use_cases: ['多服务/微服务本地环境', '拉起 Compose 服务栈'], triggers: ['项目含 docker-compose.yml', '需要本地依赖服务(数据库/缓存)'], example_call: "docker_compose_up(path='docker-compose.yml')", priority: 'medium', usage_note: '【何时用】用 docker-compose 启动多服务栈。【vs 替代】单个持续进程用 run_in_terminal；一次性命令用 run_cmd。' },
  run_worker: { category: 'orchestration', use_cases: ['大项目多模块并行实现', '独立 scope 同时开发', '把已删除的模块交给写入型 worker'], triggers: ['计划里有互不依赖的实现步骤', '多模块可按目录清晰切分 scope', '需要并行写入加速大工程'], example_call: "run_worker(description='Build API', prompt='Implement the verified API contract.', scope=['src/api'], role='backend')", priority: 'medium', usage_note: '【何时用】把互不依赖的模块交给 worker 并行实现。【vs 替代】调研用 run_subagent。' },
  run_subagent: { category: 'orchestration', use_cases: ['bug 深度取证并行', '后台调研不阻塞主线', '收集日志/复现路径/关联调用方证据', '单个聚焦文件调查不要派——主智能体直接读更快'], triggers: ['根因未明需要并行取证', '调研可后台跑不阻塞主任务', '需要独立视角审查/调研'], example_call: "run_subagent(description='Audit auth', prompt='Inspect auth and return file:line evidence.', role='research')", priority: 'medium', usage_note: '【何时用】派发后台调研/独立审查任务。【vs 替代】并行写入用 run_worker。' },
  await_subagent: { category: 'orchestration', use_cases: ['等待后台子智能体作业落定并取回报告', '下一步依赖调研结论时显式同步', '查看作业台账现状'], triggers: ['run_subagent 后台派发后需要结果', '汇合后台作业结果', '收尾前还有作业在跑', '拦截提示结果未消化'], example_call: "await_subagent(job='all')", priority: 'medium', usage_note: '【何时用】等待并取回子智能体/worker 的作业结果。' },
  generate_image: { category: 'generation', use_cases: ['图片生成'], triggers: ['需要生成图片', 'UI 素材'], example_call: "generate_image(prompt='Clean product backdrop', dest='assets/hero.png')", priority: 'medium', usage_note: '【何时用】生成图片。桌面专用。' },
  figma: { category: 'creative', use_cases: ['Figma 设计稿读取'], triggers: ['需要读取 Figma 设计', '设计稿对接'], example_call: "figma(url='https://figma.com/file/KEY/Design')", priority: 'high', usage_note: '【何时用】读取 Figma 设计稿内容。桌面/网页双端支持。' },
  screenshot: { category: 'ui_automation', use_cases: ['网页截图/视觉快照', 'UI 状态记录', '视觉回归'], triggers: ['保存界面视觉状态', '验证渲染结果', '对比 UI 变化'], example_call: "screenshot(url='http://localhost:5174')", priority: 'medium', usage_note: '【何时用】渲染网址并截图保存视觉状态/做最终视觉验收。【vs 替代】需要交互（登录/点击/表单/E2E）用 browser；和设计稿并排对比用 visual_compare。【何时不用】需要多步操作时不要用截图。' },
  computer: { category: 'desktop_automation', use_cases: ['桌面鼠标/键盘操作', '窗口管理', '剪贴板操作'], triggers: ['操控桌面应用', '模拟用户操作', '操作非浏览器窗口'], example_call: "computer(method='mouse.click', params={x:100,y:200})", priority: 'medium', usage_note: '【何时用】操控桌面应用的鼠标键盘/窗口。桌面专用。【vs 替代】浏览器内操作用 browser；系统菜单/应用跳转用 system。【何时不用】纯浏览器内交互用 browser。' },
  system: { category: 'desktop_automation', use_cases: ['系统信息查询', '窗口管理'], triggers: ['获取系统信息', '管理窗口'], example_call: "system(action='frontmost')", priority: 'low', usage_note: '【何时用】查询系统信息/管理窗口。桌面专用。' },
  read_screen: { category: 'desktop_automation', use_cases: ['屏幕内容读取', 'OCR'], triggers: ['需要读取屏幕信息'], example_call: "read_screen(ocr=false)", priority: 'low', usage_note: '【何时用】读取当前屏幕内容。桌面专用。' },
  ui_click: { category: 'desktop_automation', use_cases: ['UI 元素点击'], triggers: ['需要点击界面元素'], example_call: "ui_click(ref=12, action='press')", priority: 'low', usage_note: '【何时用】点击 UI 元素。桌面专用。' },
  visual_compare: { category: 'ui_automation', use_cases: ['设计稿与实现对比'], triggers: ['视觉回归', '设计还原检查'], example_call: "visual_compare(design='assets/design.png', url='http://localhost:5174')", priority: 'medium', usage_note: '【何时用】把实现页面与目标设计稿并排对比视觉差异（布局/间距/颜色/字体）。【vs 替代】只看当前效果不需设计稿用 screenshot；需要交互操作用 browser。' },
  design_board: { category: 'creative', use_cases: ['多方案视觉对比板'], triggers: ['设计方案展示', 'A/B 视觉对比'], example_call: "design_board(variants=[{label:'A', path:'a.png'}])", priority: 'low', usage_note: '【何时用】创建多方案视觉对比板。' },
  preview_choices: { category: 'interaction', use_cases: ['可视化选项展示'], triggers: ['需要用户做视觉选择'], example_call: "preview_choices(title='Choose layout', variants=[{name:'A', html:'<i>A</i>'}])", priority: 'low', usage_note: '【何时用】以可视化方式展示选项让用户选择。' },
  visual_explain: { category: 'creative', use_cases: ['可视化解释技术概念'], triggers: ['需要用图解释流程', '架构可视化'], example_call: "visual_explain(title='Auth flow', prompt='Login -> Token -> API')", priority: 'low', usage_note: '【何时用】用可视化方式解释技术概念/流程。' },
  research_project: { category: 'orchestration', use_cases: ['深度代码调研'], triggers: ['需要全面了解代码流'], example_call: "research_project(focus='authentication flow')", priority: 'medium', usage_note: '【何时用】派发子智能体做深度代码调研。' },
  design_research: { category: 'creative', use_cases: ['设计体系调研'], triggers: ['了解目标设计体系'], example_call: "design_research(goal='SaaS dashboard')", priority: 'medium', usage_note: '【何时用】调研目标产品的设计体系。' },
  learn_design: { category: 'creative', use_cases: ['从 URL 提取设计体系'], triggers: ['分析现有设计'], example_call: "learn_design(url='https://example.com')", priority: 'medium', usage_note: '【何时用】从指定 URL 提取设计体系规范。' },
  generate_wiki: { category: 'orchestration', use_cases: ['项目 Wiki 生成'], triggers: ['需要生成文档'], example_call: "generate_wiki(focus='architecture')", priority: 'medium', usage_note: '【何时用】派发子智能体生成项目 Wiki。' },
  game_scaffold: { category: 'generation', use_cases: ['游戏项目脚手架'], triggers: ['新建游戏项目'], example_call: "game_scaffold(engine='godot', name='space-runner')", priority: 'medium', usage_note: '【何时用】快速搭建游戏项目骨架；没有网页运行约束时默认 Godot。' },
  web_scaffold: { category: 'generation', use_cases: ['Web 项目脚手架'], triggers: ['新建 Web 项目'], example_call: "web_scaffold(name='dashboard', framework='react')", priority: 'medium', usage_note: '【何时用】快速搭建 Web 项目骨架。' },
  deploy_site: { category: 'deployment', use_cases: ['网站部署'], triggers: ['需要部署上线'], example_call: "deploy_site(name='dashboard')", priority: 'high', usage_note: '【何时用】将项目部署到线上。有安全白名单机制。' },
  worktree: { category: 'version_control', use_cases: ['Git worktree 管理'], triggers: ['多分支并行开发'], example_call: "worktree(action='list')", priority: 'low', usage_note: '【何时用】管理 git worktree 实现多分支并行。' },
  local_discovery: { category: 'location', use_cases: ['本地商户发现'], triggers: ['找附近商户/服务'], example_call: "local_discovery(query='coffee', near='current')", priority: 'low', usage_note: '【何时用】发现附近的商户/服务。非技术工具。' },
  live_environment: { category: 'location', use_cases: ['实时天气/环境查询'], triggers: ['查天气', '环境信息'], example_call: "live_environment(kind='weather')", priority: 'low', usage_note: '【何时用】查询实时天气/环境信息。' },
  current_time: { category: 'utility', use_cases: ['获取当前时间'], triggers: ['需要时间戳'], example_call: "current_time()", priority: 'low', usage_note: '【何时用】获取当前系统时间。' },
  capture_start: { category: 'networking', use_cases: ['启动网络抓包'], triggers: ['需要捕获网络请求'], example_call: "capture_start(mode='isolated_browser')", priority: 'medium', usage_note: '【何时用】启动网络抓包捕获 HTTP 请求。需要 mitmproxy。' },
  capture_flows: { category: 'networking', use_cases: ['查看抓包流量'], triggers: ['分析捕获的请求'], example_call: "capture_flows(limit=30)", priority: 'medium', usage_note: '【何时用】查看已捕获的网络流量。' },
  capture_stop: { category: 'networking', use_cases: ['停止抓包'], triggers: ['结束抓包'], example_call: "capture_stop()", priority: 'low', usage_note: '【何时用】停止网络抓包。' },
  capture_replay: { category: 'networking', use_cases: ['重放捕获的流量'], triggers: ['回放请求'], example_call: "capture_replay(id='FLOW_ID')", priority: 'low', usage_note: '【何时用】重放之前捕获的网络请求。' },
  background_monitor: { category: 'execution', use_cases: ['后台条件监控'], triggers: ['等待特定条件满足'], example_call: "background_monitor(message='Waiting for port 3000', check_type='port', pattern='3000')", priority: 'medium', usage_note: '【何时用】后台监控端口/文件/URL 等条件是否满足。' },
  automation: { category: 'desktop_automation', use_cases: ['桌面自动化通用调用'], triggers: ['复杂桌面自动化'], example_call: "automation(method='system.init', params={})", priority: 'low', usage_note: '【何时用】通用桌面自动化调用。需要 automation-server。' },
  decode_qr: { category: 'utility', use_cases: ['QR 码解码'], triggers: ['扫描 QR 码'], example_call: "decode_qr(path='assets/qr.png')", priority: 'low', usage_note: '【何时用】解码图片中的 QR 码内容。' },
  remote: { category: 'networking', use_cases: ['远程连接管理'], triggers: ['管理远程连接'], example_call: "remote(action='status')", priority: 'low', usage_note: '【何时用】管理远程 SSH/网关连接状态。' },
  start_demo: { category: 'utility', use_cases: ['启动演示模式'], triggers: ['展示功能'], example_call: "start_demo()", priority: 'low', usage_note: '【何时用】启动演示模式展示功能。' },
  stop_demo: { category: 'utility', use_cases: ['停止演示模式'], triggers: ['结束演示'], example_call: "stop_demo()", priority: 'low', usage_note: '【何时用】停止演示模式。' },

  // ── P2 补全：机械差分发现的 16 个「有 schema 但缺 TOOL_METADATA」的高价值工具 ──
  // 缺元数据 = 语义编排器 catalog 里没有【场景/触发器】关联认知 → 该工具被 under-select（工具漏斗/盲搜根因）。
  // 全部对齐 Claude Code 的「何时用 / vs 替代 / 何时不用」三段式，只补 client 侧元数据（L0 不剥、不碰主模型 prompt cache 前缀）。
  browser: { category: 'ui_automation', use_cases: ['登录/多步表单', '点击/输入/上传', 'E2E/UI 行为验证', '抓真实接口前置'], triggers: ['需要真实浏览器交互', '验证前端行为', '登录后才可见的页面'], example_call: "browser(action='navigate', url='http://127.0.0.1:5174', fresh=true)", priority: 'high', usage_note: '【何时用】需要真实浏览器交互（登录、多步表单、点击、E2E/UI 行为验证）时；默认流程 navigate→check→nodes→batch→assert。【vs 替代】只看静态布局/最终视觉验收用 screenshot；抓纯数据用 http_request；读源码用 read_file。【何时不用】读文件或抓纯数据时，绝不用浏览器一页页抄。' },
  package_search: { category: 'research', use_cases: ['查包版本与兼容性', 'latest/engines/peerDependencies 核实', '选依赖版本'], triggers: ['改 package.json/锁文件', '选版本或处理 peer 冲突', '引入新库前'], example_call: "package_search(query='axios', ecosystem='npm')", priority: 'high', usage_note: '【何时用】改 package.json/锁文件/依赖版本前，用它核实 latest、版本历史、engines、peerDependencies，别凭记忆猜版本。【vs 替代】读仓库源码用 github_repo；查打包体积用 bundlephobia_search。【何时不用】不涉及依赖版本时。' },
  get_diagnostics: { category: 'diagnostics', use_cases: ['改完代码自检', 'LSP 实时错误/警告', '定位报错行列'], triggers: ['写完/改完代码', '排查编译或类型错误', '报错但不知具体位置'], example_call: "get_diagnostics(path='src/main.ts')", priority: 'high', usage_note: '【何时用】改完代码快速自检，或排查报错时读 LSP 实时诊断（文件:行列+原因+修复方向）；这是只读证据，不运行命令。【vs 替代】运行期日志用 read_logs；跑测试/构建用 run_cmd。【何时不用】非代码文件没有诊断。' },
  read_logs: { category: 'diagnostics', use_cases: ['读终端/日志尾部', '后端/构建失败取证', '看 .log/.out/.err'], triggers: ['后端/API/构建报错', '需要真实错误原因', '持续任务输出'], example_call: "read_logs(name='dev-server', lines=200)", priority: 'high', usage_note: '【何时用】后端/API/构建失败时，读终端最新输出或日志文件尾部（只读证据，不启动新命令）。【vs 替代】编辑器实时诊断用 get_diagnostics；看持续任务运行状态用 read_terminal。【何时不用】需要跑新命令取证时用 run_cmd。' },
  run_in_terminal: { category: 'execution', use_cases: ['启动 dev server/watch', '后台守护进程/监听'], triggers: ['需要持续运行的进程', 'npm run dev / 监听服务'], example_call: "run_in_terminal(command='npm run dev', name='dev')", priority: 'high', usage_note: '【何时用】启动 dev server/watch/守护进程等持续任务；启动后用 read_logs/read_terminal 看日志与 URL，等 ready 用 background_monitor。【vs 替代】会结束的一次性命令用 run_cmd。【何时不用】一次性命令绝不用它前台硬等。' },
  find_files: { category: 'search', use_cases: ['按文件名/glob 找文件', '定位入口/配置文件'], triggers: ['知道文件名但不知路径', '需要按模式列文件'], example_call: "find_files(pattern='src/**/*.ts')", priority: 'high', usage_note: '【何时用】按文件名或 glob 模式找文件。【vs 替代】按内容找用 search；按符号定义找用 find_symbol；只能描述功能说不出关键词用 semantic_search。【何时不用】已知精确路径时直接 read_file。' },
  spawn_multiple_agents: { category: 'orchestration', use_cases: ['多视角并行调研', '大任务分角色取证'], triggers: ['大任务需要 2-5 个视角并行', '独立领域可同时调查'], example_call: "spawn_multiple_agents(task='审计架构与安全', agents=[{role:'architect', focus:'模块边界'}])", priority: 'medium', usage_note: '【何时用】大任务需要多视角并行调研（2-5 个只读角色各自取证，结果自动汇合）。【vs 替代】单个聚焦调查主智能体直接读更快；写入型并行用 run_worker；单角色调研用 run_subagent。【何时不用】单一聚焦调查不要用。' },
  search_tools: { category: 'orchestration', use_cases: ['按需装载未显示的工具', '用能力描述找工具'], triggers: ['需要的工具不在当前窗口', '知道能力说不出精确名'], example_call: "search_tools(query='数据库查询')", priority: 'medium', usage_note: '【何时用】需要某能力但当前窗口未显示该工具时，用自然语言能力描述或精确工具名请求装载。【vs 替代】已装载的工具直接调用即可。' },
  git_clone: { category: 'version_control', use_cases: ['克隆远程仓库到本地'], triggers: ['需要拉取一个远程仓库'], example_call: "git_clone(source='https://github.com/owner/repo.git', target='/abs/repo')", priority: 'medium', usage_note: '【何时用】把远程仓库克隆到一个尚不存在的绝对目录。【注意】目标路径必须尚不存在；不会弹交互式凭据。' },
  git_conflicts: { category: 'version_control', use_cases: ['列出未解决的合并冲突文件'], triggers: ['合并/变基/拉取后'], example_call: "git_conflicts()", priority: 'medium', usage_note: '【何时用】合并/变基/拉取后确认还剩哪些冲突文件要处理。【vs 替代】看整体改动状态用 git_status。' },
  git_stash_list: { category: 'version_control', use_cases: ['查看 stash 堆栈条目'], triggers: ['想找回或查看已暂存改动'], example_call: "git_stash_list()", priority: 'low', usage_note: '【何时用】列出 git stash 堆栈里现有的暂存条目。【vs 替代】恢复最近暂存用 git_stash_pop；暂存当前改动用 git_stash。' },
  git_stash_pop: { category: 'version_control', use_cases: ['取回并应用暂存改动'], triggers: ['切分支后想恢复暂存'], example_call: "git_stash_pop()", priority: 'low', usage_note: '【何时用】从 stash 堆栈取回并应用最近（或指定 index）的暂存改动。【vs 替代】查看有哪些暂存用 git_stash_list。' },
  github_repo: { category: 'research', use_cases: ['读 GitHub 仓库真实内容', 'readme/tree/file/releases/issues'], triggers: ['要看开源项目结构/源码/发布', '需要真实内容而非搜索标题'], example_call: "github_repo(owner='vitejs', repo='vite', action='readme')", priority: 'medium', usage_note: '【何时用】直接读某 GitHub 仓库的真实内容（overview/readme/tree/file/releases/issues/pulls）。【vs 替代】搜仓库/代码用 github_search；查包版本用 package_search。【何时不用】只需搜索标题列表时。' },
  gitlab_repo: { category: 'research', use_cases: ['读 GitLab 仓库真实内容'], triggers: ['要看 GitLab 项目内容/MR'], example_call: "gitlab_repo(owner='gitlab-org', repo='gitlab', action='readme')", priority: 'low', usage_note: '【何时用】直接读 GitLab.com 仓库真实内容（pulls=merge requests）。【vs 替代】GitHub 仓库用 github_repo。' },
  gitee_repo: { category: 'research', use_cases: ['读 Gitee 仓库真实内容'], triggers: ['要看国内 Gitee 项目内容'], example_call: "gitee_repo(owner='oschina', repo='git-osc', action='readme')", priority: 'low', usage_note: '【何时用】直接读 Gitee（码云）仓库真实内容。【vs 替代】GitHub 仓库用 github_repo；搜索国内项目用 gitee_search。' },
  codeberg_repo: { category: 'research', use_cases: ['读 Codeberg/Gitea 仓库真实内容'], triggers: ['要看 Codeberg 项目内容'], example_call: "codeberg_repo(owner='forgejo', repo='forgejo', action='readme')", priority: 'low', usage_note: '【何时用】直接读 Codeberg/Gitea 仓库真实内容。【vs 替代】GitHub 仓库用 github_repo。' },
});

// Compact, lazy tool documentation. These guides are emitted only when Tool Search
// returns a match, so the complete catalog never enters the model context at once.

const TOOL_EXAMPLES = Object.freeze({
  read_file: { path: "src/main.js" },
  list_dir: { path: "src" },
  search: { query: "createSession", path: "src" },
  find_files: { pattern: "src/**/*.ts" },
  web_search: { query: "official Vite migration guide" },
  web_fetch: { url: "https://vite.dev/guide/" },
  local_discovery: { query: "coffee", near: "current" },
  live_environment: { kind: "weather", latitude: 37.77, longitude: -122.42 },
  update_plan: { steps: [{ content: "Inspect current implementation", status: "in_progress" }] },
  // ask_user: the anti-guessing tool. The example must showcase options + recommended so
  // models learn to offer clickable choices (lower answer friction) instead of open questions.
  ask_user: { question: "Which deployment target should be used?", options: ["Vercel", "Docker self-host", "Static export"], recommended: 0 },
  run_subagent: { description: "Audit auth", prompt: "Inspect auth and return file:line evidence.", role: "security" },
  await_subagent: { job: "all" },
  run_worker: { description: "Build API", prompt: "Implement the verified API contract.", scope: ["src/api"], role: "backend" },
  research_project: { focus: "authentication and data flow" },
  design_research: { goal: "existing SaaS dashboard settings page" },
  learn_design: { url: "https://example.com/product" },
  recall_conversation: { query: "confirmed database decision" },
  remember: { content: "Use pnpm for this workspace; verified from pnpm-lock.yaml." },
  get_diagnostics: { path: "src/main.ts" },
  read_logs: { path: "logs/app.log", lines: 120 },
  git_status: {},
  git_diff: { staged: false },
  git_log: { max_count: 10 },
  git_blame: { path: "src/main.ts", line: 42 },
  git_stash_list: {},
  git_conflicts: {},
  gh_pr_create: { title: "Fix session expiry", body: "Summary and verification results." },
  gh_pr_view: { number: 42 },
  gh_pr_checks: { number: 42 },
  gh_actions_log: { run_id: "RUN_ID" },
  gh_pr_review_comments: { number: 42 },
  gh_pr_reply: { number: 42, body: "Fixed and verified in the latest commit." },
  read_terminal: { id: "TERMINAL_ID" },
  list_terminals: {},
  stop_terminal: { id: "TERMINAL_ID" },
  lsp_symbols: { path: "src/main.ts" },
  find_symbol: { name: "createSession" },
  knowledge_search: { query: "responsive dashboard navigation", domain: "michael-design" },
  lsp_definition: { path: "src/main.ts", line: 42, character: 8 },
  lsp_references: { path: "src/main.ts", line: 42, character: 8 },
  screenshot: { url: "http://127.0.0.1:5174", width: 1440, height: 900 },
  edit_file: { path: "src/app.ts", old_string: "const ready = false;", new_string: "const ready = true;" },
  multi_edit: { path: "src/app.ts", edits: [{ old_string: "oldValue", new_string: "newValue" }] },
  write_file: { path: "src/new-file.ts", content: "export const ready = true;\n" },
  run_cmd: { command: "npm test" },
  run_in_terminal: { command: "npm run dev" },
  delete_path: { path: "dist/stale.js" },
  move_path: { from: "src/old.ts", to: "src/new.ts" },
  create_dir: { path: "src/features/auth" },
  copy_path: { from: "config.example.json", to: "config.json" },
  format_file: { path: "src/app.ts" },
  git_commit: { message: "Fix session expiry" },
  git_branch: { action: "create", name: "codex/session-fix" },
  git_push: {},
  git_clone: { source: "https://github.com/owner/repo.git", target: "repo" },
  git_pull: {},
  git_stash: { message: "before refactor" },
  git_stash_pop: {},
  computer: { method: "screen.info", params: {} },
  system: { action: "frontmost" },
  browser: { action: "navigate", url: "http://127.0.0.1:5174", fresh: true },
  http_request: { method: "GET", url: "https://api.example.com/health" },
  download_file: { url: "https://example.com/release.zip", dest: "downloads/release.zip" },
  decode_qr: { path: "assets/qr.png" },
  remote: { action: "status" },
  generate_image: { prompt: "Clean product screenshot backdrop", dest: "assets/hero.png" },
  design_board: { variants: [{ label: "Direction A", path: "assets/a.png" }, { label: "Direction B", path: "assets/b.png" }] },
  figma: { url: "https://www.figma.com/file/FILE_KEY/Design" },
  db_query: { driver: "sqlite", url: "sqlite://./app.db", query: "SELECT * FROM users LIMIT 20" },
  start_demo: {},
  stop_demo: {},
  preview_choices: { title: "Choose layout", variants: [{ name: "A", html: "<i>A</i>", css: "" }, { name: "B", html: "<i>B</i>", css: "" }] },
  visual_explain: { title: "Session flow", prompt: "Three panels showing login, session validation, and renewal." },
  developer_community_search: { query: "Vite large monorepo migration issues", scope: "all" },
  github_search: { query: "vite plugin federation", search_type: "repositories" },
  github_repo: { owner: "vitejs", repo: "vite", action: "releases" },
  gitlab_repo: { owner: "gitlab-org", repo: "gitlab", action: "readme" },
  gitee_repo: { owner: "owner", repo: "project", action: "readme" },
  codeberg_repo: { owner: "forgejo", repo: "forgejo", action: "readme" },
  current_time: {},
  game_scaffold: { engine: "godot", name: "space-runner" },
  generate_3d: { prompt: "Low-poly sci-fi crate", name: "sci-fi-crate" },
  generate_sound: { prompt: "Short metallic UI confirmation", name: "confirm" },
  generate_music: { prompt: "Looping calm strategy-game theme", name: "strategy-loop" },
  generate_voice: { text: "Mission complete.", name: "mission-complete" },
  auto_rig: { model_path: "assets/character.glb", name: "hero-rig" },
  generate_motion: { prompt: "Natural walk cycle", name: "walk" },
  generate_texture: { prompt: "Seamless worn steel", name: "worn-steel" },
  search_game_assets: { query: "CC0 low-poly spaceship" },
  download_asset: { url: "https://example.com/asset.glb", name: "spaceship.glb" },
  visual_compare: { design: "assets/design.png", url: "http://127.0.0.1:5174" },
  web_scaffold: { name: "product-dashboard", framework: "react" },
  read_screen: { ocr: false },
  ui_click: { ref: 12, action: "press" },
  generate_wiki: { focus: "architecture and core workflows" },
  worktree: { action: "list" },
  semantic_search: { query: "where login sessions are validated", top_k: 8 },
  deploy_site: { name: "product-dashboard" },
  tor_request: { method: "GET", url: "http://example.onion/" },
  capture_start: { mode: "isolated_browser" },
  automation: { method: "system.init", params: {} },
  capture_flows: { include_body: false, limit: 30 },
  capture_stop: {},
  capture_replay: { id: "FLOW_ID" },
  background_monitor: { message: "Waiting for the dev server", check_type: "port", pattern: "3000" },
});

const FIELD_VALUES = Object.freeze({
  path: "src/main.ts", from: "src/old.ts", to: "src/new.ts", dest: "output/result.txt",
  url: "https://example.com", query: "current project requirement", pattern: "src/**/*.ts",
  command: "npm test", content: "verified project fact", prompt: "Inspect the real project evidence.",
  description: "Focused task", title: "Focused task", name: "example", owner: "owner", repo: "repo",
  message: "Verified change", text: "example text", old_string: "oldValue", new_string: "newValue",
  source: "https://github.com/owner/repo.git", driver: "sqlite", action: "status", method: "GET",
  focus: "architecture", goal: "production-ready implementation", question: "Which verified option fits the constraints?",
  tracking_number: "TRACKING_NUMBER", task_id: "TASK_ID", id: "ID", ref: 1, line: 1, character: 1,
  latitude: 37.77, longitude: -122.42, width: 1440, height: 900, limit: 10, max_results: 8,
});

function safeEnumValue(schema) {
  const values = Array.isArray(schema?.enum) ? schema.enum : [];
  return values.find((value) => (typeof value === "string" && /^[A-Za-z0-9._:-]{1,48}$/.test(value))
    || typeof value === "number" || typeof value === "boolean");
}

function exampleValue(field, schema, depth = 0) {
  const enumValue = safeEnumValue(schema);
  if (enumValue !== undefined) return enumValue;
  if (Object.prototype.hasOwnProperty.call(FIELD_VALUES, field)) return FIELD_VALUES[field];
  const type = schema?.type || (schema?.properties ? "object" : "string");
  if (type === "boolean") return true;
  if (type === "integer" || type === "number") {
    const minimum = Number(schema?.minimum);
    return Number.isFinite(minimum) ? minimum : 1;
  }
  if (type === "array") {
    if (depth >= 2) return [];
    return [exampleValue(field.replace(/s$/, "") || "item", schema?.items || {}, depth + 1)];
  }
  if (type === "object") {
    if (depth >= 2) return {};
    return compactToolExampleArgs({ function: { name: "", parameters: schema } }, depth + 1);
  }
  return field ? `example_${field}`.slice(0, 36) : "example";
}

function requiredKeys(parameters) {
  const keys = [...(Array.isArray(parameters?.required) ? parameters.required : [])];
  const branch = [...(parameters?.anyOf || []), ...(parameters?.oneOf || [])]
    .find((item) => Array.isArray(item?.required));
  if (branch) for (const key of branch.required) if (!keys.includes(key)) keys.push(key);
  return keys.slice(0, 5);
}

export function compactToolExampleArgs(schema, depth = 0) {
  const name = String(schema?.function?.name || "");
  if (depth === 0 && Object.prototype.hasOwnProperty.call(TOOL_EXAMPLES, name)) {
    return TOOL_EXAMPLES[name];
  }
  const parameters = schema?.function?.parameters || schema?.parameters || {};
  const properties = parameters?.properties || {};
  const out = {};
  for (const key of requiredKeys(parameters)) out[key] = exampleValue(key, properties[key] || {}, depth);
  return out;
}

function compactScenario(description, maxChars) {
  const cleaned = String(description || "需要该能力时使用")
    .replace(/<[^>]+>/g, " ")
    .replace(/[`*_#>|]/g, "")
    .replace(/[⚠️✅❌🔍🎨📦🚀]+/gu, "")
    .replace(/\s+/g, " ")
    .trim();
  const first = (cleaned.split(/[。；\n]/)[0] || cleaned || "需要该能力时使用").trim();
  return first.length > maxChars ? `${first.slice(0, Math.max(1, maxChars - 1))}…` : first;
}

export function compactToolGuide(schema, maxChars = 180) {
  const name = String(schema?.function?.name || "unknown_tool").replace(/[^A-Za-z0-9_-]/g, "").slice(0, 64) || "unknown_tool";
  const args = compactToolExampleArgs(schema);
  const invocation = `${name}(${JSON.stringify(args)})`;
  const fixed = `${name}｜场景:｜例:${invocation}`;
  const scenario = compactScenario(schema?.function?.description, Math.max(12, maxChars - fixed.length));
  // Never truncate JSON in the invocation. A future external tool with unusually
  // long required field names may exceed the target, but still gets a usable guide.
  return `${name}｜场景:${scenario}｜例:${invocation}`;
}

/**
 * 自动推断工具元数据（兜底策略）
 * 当某个工具在 TOOL_METADATA 中没有定义时，根据工具名进行启发式推断，
 * 保证每个工具在 catalog 中至少有【触发器】提示。
 */
export function autoEnrichToolMetadata(entry) {
  const name = String(entry?.name || '');
  const displayName = name.toLowerCase();
  if (!displayName) return null;

  const inferenceRules = [
    { patterns: ['search', 'query', 'find', 'discover'], category: 'research', default_triggers: ['需要查找信息', '调研技术'] },
    { patterns: ['debug', 'error', 'log', 'diagnostic'], category: 'diagnostics', default_triggers: ['遇到问题', '调试排查'] },
    { patterns: ['profiler', 'benchmark', 'performance'], category: 'analysis', default_triggers: ['性能问题', '慢查询'] },
    { patterns: ['browser', 'screenshot', 'visual', 'computer', 'ui_'], category: 'ui_automation', default_triggers: ['前端交互', '界面测试'] },
    { patterns: ['db_', 'sql', 'database', 'migration'], category: 'data_layer', default_triggers: ['数据库操作', '数据结构变更'] },
    { patterns: ['git_', 'gh_', 'commit', 'branch'], category: 'version_control', default_triggers: ['版本控制', '代码提交与 PR'] },
    { patterns: ['generate_', 'design', 'figma'], category: 'creative', default_triggers: ['设计与素材生成', '视觉产出'] },
    { patterns: ['terminal', 'run_', 'cmd', 'exec'], category: 'execution', default_triggers: ['执行命令', '运行验证'] },
    { patterns: ['capture', 'http', 'request', 'fetch'], category: 'networking', default_triggers: ['网络请求取证', '接口测试'] },
  ];

  for (const rule of inferenceRules) {
    if (rule.patterns.some((p) => displayName.includes(p))) {
      return {
        category: rule.category,
        use_cases: [],
        triggers: rule.default_triggers,
        example_call: '', // 通用模板没有真实参数价值，留空由 TOOL_EXAMPLES 兼并
      };
    }
  }

  return null; // 无法推断
}

/**
 * 生成单行增强 catalog 条目：基础 name/description/inputs 之外，
 * 追加【场景】【触发器】【示例】三段式说明，帮 AI 建立工具↔场景关联认知。
 * 供 main.js 的编排器/评审器 catalog 生成使用。
 */
export function enrichedCatalogLine(entry) {
  const inputs = Array.isArray(entry?.inputs) ? entry.inputs.join(",") : "";
  const required = Array.isArray(entry?.required) && entry.required.length ? ` required:${entry.required.join(",")}` : "";
  let line = `${entry?.name}\t${entry?.description || "（无描述）"}\t${inputs}${required}`;

  const meta = TOOL_METADATA[entry?.name] || autoEnrichToolMetadata(entry);
  if (!meta) return line;

  if (Array.isArray(meta.use_cases) && meta.use_cases.length > 0) {
    line += `\t【场景】${meta.use_cases.join('、')}`;
  }
  if (Array.isArray(meta.triggers) && meta.triggers.length > 0) {
    line += `\t【触发器】${meta.triggers.join('、')}`;
  }
  if (meta.usage_note) {
    line += `\t【注意】${meta.usage_note}`;
  }
  if (meta.example_call) {
    line += `\t【示例】${meta.example_call}`;
  }
  return line;
}

export { TOOL_METADATA };
