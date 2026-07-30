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
    usage_note: '🛡️ GitHub 先验纪律要求：遇到陌生技术栈时，优先调用此工具而非盲目实现'
  },
  developer_community_search: {
    category: 'research',
    use_cases: ['技术优缺点讨论', '踩坑经验分享', '方案评估'],
    triggers: ['陌生技术栈', '性能问题', '需要社区经验', '评估技术方案'],
    example_call: "developer_community_search(query='Vite monorepo migration issues', scope='all')",
    priority: 'high'
  },
  web_search: {
    category: 'research',
    use_cases: ['官方文档查找', '错误信息搜索', '技术教程查询', '联网查最新信息'],
    triggers: ['需要了解外部知识', '遇到报错', '查找官方文档', '用户要求联网/上网搜索', '需要时效性/最新信息'],
    example_call: "web_search(query='Vite official migration guide')",
    priority: 'high'
  },
  
  // 调试诊断类工具
  debugger: {
    category: 'diagnostics',
    use_cases: ['断点调试', '变量追踪', '调用栈分析'],
    triggers: ['运行时错误', '逻辑 bug', '需要追踪执行流'],
    example_call: "debugger(error_log='Uncaught TypeError at line 42')",
    priority: 'high'
  },
  search: {
    category: 'diagnostics',
    use_cases: ['错误码搜索', '日志关键词查找', '代码模式匹配'],
    triggers: ['遇到报错', '需要定位问题', '搜索错误信息'],
    example_call: "search(query='TypeError.*line 42', path='src')",
    priority: 'high'
  },
  read_logs: {
    category: 'diagnostics',
    use_cases: ['查看应用日志', '排查生产环境 bug', '监控异常模式'],
    triggers: ['日志文件存在', '需要追溯历史行为', '系统异常'],
    example_call: "read_logs(path='logs/app.log', lines=200)",
    priority: 'medium'
  },
  
  // 性能分析类工具
  profiler: {
    category: 'analysis',
    use_cases: ['接口响应时间分析', 'CPU/内存瓶颈定位', '性能优化验证'],
    triggers: ['响应慢', '性能卡顿', '超时问题', '需要性能基准'],
    example_call: "profiler(endpoint='/api/users')",
    priority: 'high'
  },
  db_query: {
    category: 'data_layer',
    use_cases: ['数据库结构检查', '慢查询分析', '数据验证'],
    triggers: ['数据库操作', '数据结构变更', '需要 inspect schema', '查询慢'],
    example_call: "db_query(driver='sqlite', query='EXPLAIN SELECT * FROM users')",
    priority: 'high'
  },
  
  // UI 自动化类工具
  browser_launch: {
    category: 'ui_automation',
    use_cases: ['前端交互测试', 'UI 状态验证', '页面截图'],
    triggers: ['前端界面', '用户交互', '需要验证视觉效果'],
    example_call: "browser(action='navigate', url='http://localhost:5174', fresh=true)",
    priority: 'medium'
  },
  screenshot: {
    category: 'ui_automation',
    use_cases: ['界面快照', '视觉回归测试', 'UI 状态记录'],
    triggers: ['需要保存界面状态', '验证渲染结果', '对比 UI 变化'],
    example_call: "screenshot(url='http://localhost:5174', width=1440, height=900)",
    priority: 'low'
  },
  
  // 数据库操作类工具
  db_migrate: {
    category: 'data_layer',
    use_cases: ['数据迁移', '表结构变更', 'Schema 更新'],
    triggers: ['需要修改数据库结构', '数据格式转换', '版本升级'],
    example_call: "db_migrate(script='./migrations/20240101_add_users.sql')",
    priority: 'high'
  },
  backup_database: {
    category: 'data_layer',
    use_cases: ['变更前备份', '数据恢复准备', '安全操作前置'],
    triggers: ['重大变更前', '数据迁移前', '危险操作前'],
    example_call: "backup_database(source='app.db', dest='./backups/app.backup')",
    priority: 'high'
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
    priority: 'medium'
  },
  
  // Git 相关工具
  git_branch: {
    category: 'version_control',
    use_cases: ['功能分支创建', '隔离实验代码', 'PR 准备'],
    triggers: ['开始新功能', '需要隔离改动', '准备提交代码'],
    example_call: "git_branch(action='create', name='feature/session-fix')",
    priority: 'medium'
  },
  git_commit: {
    category: 'version_control',
    use_cases: ['阶段性保存', '代码归档', 'PR 前提'],
    triggers: ['完成一个小阶段', '需要保存当前状态', '准备推送'],
    example_call: "git_commit(message='Fix session expiry issue')",
    priority: 'medium'
  },
  
  // 文件系统操作
  read_file: {
    category: 'file_io',
    use_cases: ['代码阅读', '配置检查', '文档理解'],
    triggers: ['需要理解代码', '查看配置文件', '阅读文档'],
    example_call: "read_file(path='src/main.js')",
    priority: 'critical'
  },
  list_dir: {
    category: 'file_io',
    use_cases: ['项目结构探索', '文件发现', '目录遍历'],
    triggers: ['陌生项目', '需要找文件', '了解目录结构'],
    example_call: "list_dir(path='src')",
    priority: 'critical'
  },
  
  // Web 服务与 API
  http_request: {
    category: 'networking',
    use_cases: ['API 健康检查', '接口测试', '外部服务调用'],
    triggers: ['需要调用 API', '验证服务状态', '后端交互'],
    example_call: "http_request(method='GET', url='https://api.example.com/health')",
    priority: 'medium'
  },
  
  // 性能分析类工具（新增）
  performance_profile: {
    category: 'performance_analysis',
    use_cases: ['前端性能检测', '页面加载慢问题排查', 'CPU 内存监控'],
    triggers: ['页面响应慢', '性能卡顿', '需要性能基准', '定位渲染瓶颈'],
    example_call: "performance_profile(url='http://localhost:5174', metrics='both', timeoutSeconds=30)",
    priority: 'high'
  },
  
  // 规范解析类工具 (新增)
  openapi_parser: {
    category: 'specification_parsing',
    use_cases: ['API 端点清单提取', 'Swagger 规范审查', '生成 curl 示例模板'],
    triggers: ['需要查看可用 API', 'OpenAPI/Swagger 文档', '接口对接开发'],
    example_call: "openapi_parser(url='./openapi.json', outputFormat='list')",
    priority: 'medium'
  },
  
  // 子智能体编排类工具 (P2.1 异步作业 + #46 拆分并行)
  run_worker: {
    category: 'orchestration',
    use_cases: ['大项目多模块并行实现', '独立 scope 同时开发', '把已明确契约的模块交给写入型 worker'],
    triggers: ['计划里有互不依赖的实现步骤', '多模块可按目录清晰切分 scope', '需要并行写入加速大工程'],
    example_call: "run_worker(description='Build API', prompt='Implement the verified API contract.', scope=['src/api'], role='backend')",
    priority: 'medium'
  },
  run_subagent: {
    category: 'orchestration',
    use_cases: ['bug 深度取证并行', '后台调研不阻塞主线', '收集日志/复现路径/关联调用方证据', '单个聚焦文件调查不要派——主智能体直接读更快'],
    triggers: ['根因未明需要并行取证', '调研可后台跑不阻塞主任务', '需要独立视角审查/调研'],
    example_call: "run_subagent(description='Audit auth', prompt='Inspect auth and return file:line evidence.', role='research')",
    priority: 'medium'
  },
  await_subagent: {
    category: 'orchestration',
    use_cases: ['等待后台子智能体作业落定并取回报告', '下一步依赖调研结论时显式同步', '查看作业台账现状'],
    triggers: ['run_subagent 后台派发后需要结果', '汇合后台作业结果', '收尾前还有作业在跑', '拦截提示结果未消化'],
    example_call: "await_subagent(job='all')",
    priority: 'medium'
  },
  
  // 测试骨架生成类工具 (新增)
  generate_test_cases: {
    category: 'code_quality',
    use_cases: ['写完功能要补测试', '为导出函数/类生成测试骨架', '补齐正常/边界/错误用例框架'],
    triggers: ['需要补单元测试', '新功能缺测试覆盖', '生成测试文件模板'],
    example_call: "generate_test_cases(path='src/utils.js', framework='auto')",
    priority: 'medium'
  },
  
  // 容器编排执行类工具 (新增)
  docker_compose_up: {
    category: 'execution',
    use_cases: ['启动多服务/微服务本地环境', '拉起 Compose 服务栈', '容器启动失败自愈提示'],
    triggers: ['项目含 docker-compose.yml', '需要本地依赖服务 (数据库/缓存)', '启动容器服务栈'],
    example_call: "docker_compose_up(path='docker-compose.yml', detach=true)",
    priority: 'medium'
  },
    
  // 实时新闻聚合类工具 (新增)
  realtime_news_feed: {
    category: 'information_gathering',
    use_cases: ['了解技术近期动态', '版本发布跟踪', '社区评价收集', '技术风向调研'],
    triggers: ['想了解某技术的最新讨论', '关注版本发布动态', '收集社区反馈'],
    example_call: "realtime_news_feed(topic='Rust 1.80', sources='all', maxResults=15)",
    priority: 'high'
  },
    
  // 代码生成与修改
  edit_file: {
    category: 'code_editing',
    use_cases: ['小范围修改', '重构代码', '修复 bug'],
    triggers: ['已知修改位置', '替换特定代码块', '精确修改'],
    example_call: "edit_file(path='src/app.ts', old_string='const ready = false;', new_string='const ready = true;')",
    priority: 'high'
  },
  write_file: {
    category: 'code_editing',
    use_cases: ['创建新文件', '生成配置', '写入测试结果'],
    triggers: ['新建文件', '生成代码', '保存输出'],
    example_call: "write_file(path='src/new-feature.ts', content='export const x = 1;')",
    priority: 'high'
  },
  run_cmd: {
    category: 'execution',
    use_cases: ['运行测试', '构建项目', '执行脚本'],
    triggers: ['需要执行程序', '运行命令', '验证构建'],
    example_call: "run_cmd(command='npm test')",
    priority: 'high'
  }
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
  live_markets: { kind: "exchange_rate", base: "USD", quote: "CNY" },
  live_flights: { latitude: 37.77, longitude: -122.42 },
  road_environment: { kind: "overview", near: "current" },
  track_shipment: { tracking_number: "TRACKING_NUMBER", carrier: "auto" },
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
  computer: { action: "nodes" },
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
  game_scaffold: { engine: "threejs", name: "space-runner" },
  generate_3d: { prompt: "Low-poly sci-fi crate", name: "sci-fi-crate" },
  generate_sound: { prompt: "Short metallic UI confirmation", name: "confirm" },
  generate_music: { prompt: "Looping calm strategy-game theme", name: "strategy-loop" },
  generate_voice: { text: "Mission complete.", name: "mission-complete" },
  auto_rig: { task_id: "TASK_ID", name: "hero-rig" },
  generate_motion: { task_id: "TASK_ID", prompt: "Natural walk cycle", name: "walk" },
  generate_texture: { prompt: "Seamless worn steel", name: "worn-steel" },
  search_game_assets: { query: "CC0 low-poly spaceship" },
  download_asset: { url: "https://example.com/asset.glb", name: "spaceship.glb" },
  visual_compare: { design: "assets/design.png", url: "http://127.0.0.1:5174" },
  web_scaffold: { name: "product-dashboard", framework: "react" },
  read_screen: { ocr: false },
  ui_click: { ref: 12, action: "press" },
  shop_catalog: { query: "wireless keyboard", url: "https://shop.example.com" },
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
  debate: { question: "Should this existing service use sessions or JWT?", perspectives: ["security", "operations"] },
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
