// LSP 环境探针那条路上的三道门。它们守的是**整个 IDE 会不会卡死**，不是某个补全好不好用。
//
// 病理链（2026-08-27 实测复现）：
//   lsp_python_env_symbols 抱着**全进程唯一**的 PY_CACHE 锁去跑两次无超时的子进程
//   → 一个慢/卡住的解释器把所有 Python 补全串成一队
//   → 而这些命令是 `#[tauri::command(async)]` 套在同步 fn 上，tauri 的宏把同步体直接内联进
//     async 块交给 tokio::spawn，也就是**在 worker 线程上就地阻塞**（不是 spawn_blocking）
//   → 攒够 CPU 核数个卡住的调用，整个 runtime 被饿死；而 git、存盘、终端、AI 这些同样是
//     command(async) 的同步 fn（全仓 83 个）共用这一个池，一起停止响应。
//
// Rust 那两条（锁不跨子进程 / 子进程有上限）是**行为**测试，在 src-tauri/src/lsp.rs 里，
// 用假解释器量墙钟。这里只守前端和 Rust 的接线形状。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { CODE as SRC, fnSource as extractFn } from "./helpers/source.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const LSP_RS = readFileSync(join(HERE, "..", "src-tauri", "src", "lsp.rs"), "utf8");

test("打开文件时的 Python 探针必须有语言门——否则每开一个 .js 都 fork 一个解释器", () => {
  // 上一版 _onFileOpened 无条件对任何带 import 的文件发这一发：打开 .js/.ts/.rs/.java
  // 同样会 fork 一个项目 venv 的 python 去 `import react`。那必然失败 → 模块名不进
  // _loadedModuleApis → **去重永远不生效**，每打开一次同样的文件就再发一次。
  const opened = extractFn("_onFileOpened", { code: true });
  assert.match(opened, /langId === "python"/,
    "打开文件那条没有语言门 —— 非 Python 文件也会 fork 解释器，且去重不生效");
  const gate = opened.indexOf('langId === "python"');
  const extract = opened.indexOf("_extractImportedModules");
  assert.ok(gate > 0 && gate < extract, "语言门必须排在取 import 列表之前");
  // 非 Python 不能被落下：onDidChangeModel（打开文件同样触发）那条按 langId 分派。
  assert.match(SRC, /_refreshModuleApis\(model\), 2000\)/,
    "非 Python 语言的探针入口没了 —— 这条改动就从「省一次 fork」变成「砍掉一个功能」");
});

test("跑外部工具链的 LSP 命令必须走 spawn_blocking，否则它们就地占住 tokio worker", () => {
  // #[tauri::command(async)] 对同步 fn 的语义是「丢到异步 runtime 上」，不是「丢到阻塞
  // 线程池上」。这五个都在 fork/exec/wait 解释器、node、go，一次冷启轻松几秒。
  for (const name of ["lsp_detect_python", "lsp_python_env_symbols", "lsp_node_env_symbols",
                      "lsp_go_env_symbols", "lsp_lang_env_symbols"]) {
    const re = new RegExp(`pub async fn ${name}\\([\\s\\S]{0,400}?spawn_blocking\\(move \\|\\| ${name}_blocking\\(`);
    assert.match(LSP_RS, re,
      `${name} 没走 spawn_blocking —— 它会在 worker 线程上就地阻塞，攒够核数个就把整个 IDE 冻住`);
  }
});

test("Python 探针的子进程有上限，而且锁不跨越它", () => {
  // 判据落在 Rust 源码的**接线**上；「真的不串行 / 真的会超时」由 lsp.rs 里那两条
  // 行为测试用假解释器量墙钟守着（python_probe_does_not_serialize… / …gives_up…）。
  assert.match(LSP_RS, /wait_with_timeout_pub\(child, py_script_timeout\(\)\)/,
    "run_python_script 又变回裸 output()：一个 import 时连网的包就能把全局锁永久按住");
  assert.doesNotMatch(LSP_RS, /let output = cmd\.output\(\)\.ok\(\)\?;/,
    "还留着无超时的 output() 调用");
  const fn = LSP_RS.slice(LSP_RS.indexOf("fn lsp_python_env_symbols_blocking"));
  const body = fn.slice(0, fn.indexOf("\n}\n"));
  const lockOnce = body.indexOf("py_cache().lock()");
  const subprocess = body.indexOf("run_python_script");
  assert.ok(lockOnce >= 0 && subprocess > 0, "函数形状变了，这条门失去锚点");
  assert.match(body, /\/\/ ② 锁外：跑子进程/,
    "「锁外跑子进程」那段没了 —— 锁又跨回子进程调用了");
  assert.ok((body.match(/py_cache\(\)\.lock\(\)/g) || []).length >= 2,
    "只 lock 了一次 = 多半又是一把长活的锁；正确形状是「锁内读、锁外跑、锁内写回」");
  // 失败不许建缓存条目：上一版 get_or_insert_with 无条件建了 fetched_at=now 的空条目，
  // 于是一次失败之后 300 秒里每个调用者都拿到 cached:true 和 0 个模块。
  assert.match(body, /if !mods\.is_empty\(\) \{[\s\S]{0,200}?py_cache\(\)\.lock\(\)/,
    "失败/超时时仍会建缓存条目 —— 一张空模块表会被钉住 300 秒");
});
