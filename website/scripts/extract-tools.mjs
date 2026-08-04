/**
 * Reads the IDE's real tool catalog out of src/main.js and writes public/tools.json.
 *
 * The site renders whatever this emits, so adding a tool to `_buildAgentToolSchemas`
 * and rebuilding is all it takes for the gallery to pick it up — there is no second
 * list to keep in sync.
 */
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const SOURCE = resolve(here, "../../src/main.js");
const OUT = resolve(here, "../public/tools.json");

const src = readFileSync(SOURCE, "utf8");

// Tool schemas are declared as `{ type: "function", function: { name: "…" } }`.
const names = [];
for (const m of src.matchAll(/function:\s*\{\s*name:\s*"([a-z_0-9]+)"/g)) {
  if (!names.includes(m[1])) names.push(m[1]);
}
if (names.length < 50) {
  throw new Error(
    `extract-tools: only found ${names.length} tools — the schema shape in src/main.js probably changed.`,
  );
}

/** Group by what the tool actually touches. Order matters: first match wins. */
const GROUPS = [
  ["Files", /^(read_file|write_file|edit_file|multi_edit|list_dir|create_dir|copy_path|move_path|delete_path|format_file|read_logs|download_file)$/],
  // Searching YOUR repository, kept apart from the research lookups below.
  ["Code search", /^(search|find_files|semantic_search|find_symbol|search_tools|deep_search|sourcegraph_search)$/],
  ["Code intelligence", /^(lsp_|get_diagnostics|generate_test_cases|visual_explain)/],
  ["Git & GitHub", /^(git_|gh_|gitlab_|gitee_|codeberg_)/],
  ["Terminal & system", /^(run_cmd|run_in_terminal|read_terminal|list_terminals|stop_terminal|system|worktree|docker_compose_up|background_monitor)$/],
  ["Agents", /^(run_subagent|await_subagent|spawn_multiple_agents|run_worker|debate|research_project|generate_wiki|update_plan|ask_user|recall_conversation|remember)$/],
  ["Web & browser", /^(browser|web_fetch|web_search|http_request|tor_request|screenshot|read_screen|ui_click|automation|capture_|decode_qr|remote|start_demo|stop_demo)/],
  ["Design & media", /^(design_|learn_design|generate_image|visual_compare|preview_choices|iconify_search|color_search)/],
  ["Data", /^(db_query|openapi_parser|performance_profile|live_|road_environment|track_shipment|shop_catalog|realtime_news_feed|current_time|local_discovery)/],
  ["Deploy", /^(deploy_site)$/],
  // Everything else ending in _search is a research lookup against a public corpus.
  ["Research", /_search$|^github_repo$|^github_trending$|^gitlab_repo$|^gitee_repo$|^codeberg_repo$/],
];

// Tools the app deliberately withholds from the browser build (see the desktopOnly
// set in _buildAgentToolSchemas). The gallery marks them so nobody is told a demo
// failed when the product is simply refusing to expose a desktop capability.
const DESKTOP_ONLY = new Set(
  (src.match(/const desktopOnly = new Set\(\[([\s\S]*?)\]\)/)?.[1] ?? "")
    .split(",")
    .map((s) => s.trim().replace(/^["']|["']$/g, ""))
    .filter(Boolean),
);

const tools = names.map((name) => {
  const group = GROUPS.find(([, re]) => re.test(name))?.[0] ?? "Knowledge";
  return { name, group, desktopOnly: DESKTOP_ONLY.has(name) };
});

const payload = {
  generatedFrom: "ide/src/main.js · _buildAgentToolSchemas",
  generatedAt: null, // deliberately unset: a timestamp would churn the build output
  count: tools.length,
  groups: [...new Set(tools.map((t) => t.group))].sort(),
  tools,
};

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, JSON.stringify(payload, null, 2) + "\n");
console.log(`extract-tools: ${tools.length} tools → public/tools.json`);
