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
  ask_user: { question: "Which deployment target should be used?" },
  run_subagent: { description: "Audit auth", prompt: "Inspect auth and return file:line evidence.", role: "security" },
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
