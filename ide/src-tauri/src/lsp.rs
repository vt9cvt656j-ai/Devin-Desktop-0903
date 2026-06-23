use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use crate::process_util;

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[allow(dead_code)]
pub enum LspEvent {
    Message { data: String },
    Started { lang: String },
    Error { message: String },
    Stopped { lang: String },
}

struct LspProcess {
    child: Child,
    stdin_tx: std::sync::mpsc::Sender<String>,
}

#[derive(Default)]
pub struct LspManager {
    inner: Mutex<HashMap<String, LspProcess>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LspServerConfig {
    pub lang: String,
    pub command: String,
    pub args: Vec<String>,
    pub root_uri: String,
}

const KNOWN_SERVERS: &[(&str, &str, &[&str])] = &[
    ("typescript", "typescript-language-server", &["--stdio"]),
    ("javascript", "typescript-language-server", &["--stdio"]),
    ("rust", "rust-analyzer", &[]),
    ("python", "pyright-langserver", &["--stdio"]),
    ("go", "gopls", &["serve"]),
    ("c", "clangd", &[]),
    ("cpp", "clangd", &[]),
    ("objective-c", "clangd", &[]),
    ("html", "vscode-html-language-server", &["--stdio"]),
    ("css", "vscode-css-language-server", &["--stdio"]),
    ("json", "vscode-json-language-server", &["--stdio"]),
    // Extended language coverage. Each only activates when its server binary is
    // installed; otherwise the UI offers a one-click install hint.
    ("java", "jdtls", &[]),
    ("ruby", "solargraph", &["stdio"]),
    ("php", "intelephense", &["--stdio"]),
    ("lua", "lua-language-server", &[]),
    ("shell", "bash-language-server", &["start"]),
    ("yaml", "yaml-language-server", &["--stdio"]),
    ("csharp", "omnisharp", &["-lsp"]),
    ("kotlin", "kotlin-language-server", &[]),
    ("swift", "sourcekit-lsp", &[]),
    ("dart", "dart", &["language-server", "--protocol=lsp"]),
    ("elixir", "elixir-ls", &[]),
    ("clojure", "clojure-lsp", &[]),
    ("scala", "metals", &[]),
    ("hcl", "terraform-ls", &["serve"]),
    ("graphql", "graphql-lsp", &["server", "-m", "stream"]),
    ("dockerfile", "docker-langserver", &["--stdio"]),
    ("vue", "vue-language-server", &["--stdio"]),
];

fn find_server(lang: &str) -> Option<(&'static str, &'static [&'static str])> {
    KNOWN_SERVERS
        .iter()
        .find(|(l, _, _)| *l == lang)
        .map(|(_, cmd, args)| (*cmd, *args))
}

fn prune_stopped(inner: &mut HashMap<String, LspProcess>) {
    inner.retain(|_, proc| match proc.child.try_wait() {
        Ok(Some(_)) => false,
        Ok(None) => true,
        Err(_) => false,
    });
}

fn extract_method(json: &str) -> String {
    if let Some(start) = json.find("\"method\"") {
        let rest = &json[start + 9..];
        if let Some(q1) = rest.find('"') {
            let inner = &rest[q1 + 1..];
            if let Some(q2) = inner.find('"') {
                return inner[..q2].to_string();
            }
        }
    }
    if json.contains("\"result\"") { return "response".to_string(); }
    "?".to_string()
}

fn encode_lsp_message(content: &str) -> String {
    format!(
        "Content-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}


/// Strip a `file://` prefix from a root URI and URL-decode to get a real path.
fn workspace_dir_from_uri(uri: &str) -> Option<String> {
    let trimmed = uri.strip_prefix("file://").unwrap_or(uri);
    if trimmed.is_empty() {
        return None;
    }
    let decoded = percent_decode(trimmed);
    if decoded.is_empty() { None } else { Some(decoded) }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[tauri::command]
pub fn lsp_start(
    state: State<LspManager>,
    config: LspServerConfig,
    on_event: Channel<LspEvent>,
) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    prune_stopped(&mut inner);

    if inner.contains_key(&config.lang) {
        return Err(format!("LSP for '{}' is already running", config.lang));
    }
    if inner.len() >= process_util::MAX_CHILD_PROCESSES {
        return Err("too many language servers running; stop one first".into());
    }

    let command = if config.command.is_empty() {
        let (cmd, _) = find_server(&config.lang).ok_or_else(|| {
            format!("no known LSP server for '{}'; provide a custom command", config.lang)
        })?;
        cmd.to_string()
    } else {
        config.command.clone()
    };
    let args: Vec<String> = if config.command.is_empty() {
        let (_, default_args) = find_server(&config.lang).ok_or_else(|| {
            format!("no known LSP server for '{}'; provide a custom command", config.lang)
        })?;
        default_args.iter().map(|arg| (*arg).to_string()).collect()
    } else {
        config.args.clone()
    };

    let ws = workspace_dir_from_uri(&config.root_uri);
    #[cfg(not(windows))]
    let resolved = process_util::resolve_command(&command, ws.as_deref());
    #[cfg(windows)]
    let resolved = command.clone();

    // Detect Node.js shebang scripts and run them through node directly,
    // because the kernel's shebang handler uses the parent process PATH
    // which is minimal when launched from macOS Finder.
    #[cfg(not(windows))]
    let (actual_cmd, extra_args) = {
        if let Ok(content) = std::fs::read_to_string(&resolved) {
            if content.starts_with("#!/usr/bin/env node") || content.starts_with("#!/usr/bin/env -S node") {
                let node = process_util::resolve_command("node", ws.as_deref());
                (node, vec![resolved.clone()])
            } else {
                (resolved.clone(), vec![])
            }
        } else {
            (resolved.clone(), vec![])
        }
    };
    #[cfg(windows)]
    let (actual_cmd, extra_args) = (resolved.clone(), Vec::<String>::new());

    let mut builder = Command::new(&actual_cmd);
    builder
        .args(&extra_args)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(ref ws_dir) = ws {
        builder.current_dir(ws_dir);
        #[cfg(not(windows))]
        builder.env("PATH", process_util::augmented_path(Some(ws_dir)));
    } else {
        #[cfg(not(windows))]
        builder.env("PATH", process_util::augmented_path(None));
    }
    tracing::info!(
        "[lsp] spawning: cmd={actual_cmd:?} extra={extra_args:?} args={args:?} resolved={resolved:?}"
    );
    let mut child = builder
        .spawn()
        .map_err(|e| format!("failed to start '{}' (resolved={}, actual={}): {}", command, resolved, actual_cmd, e))?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    let (stdin_tx, stdin_rx) = std::sync::mpsc::channel::<String>();

    let mut stdin_handle = child.stdin.take().ok_or("no stdin")?;
    let send_lang = config.lang.clone();
    std::thread::spawn(move || {
        while let Ok(msg) = stdin_rx.recv() {
            let method = extract_method(&msg);
            tracing::debug!("[lsp-{send_lang}] → {method}");
            let encoded = encode_lsp_message(&msg);
            if stdin_handle.write_all(encoded.as_bytes()).is_err() {
                tracing::warn!("[lsp-{send_lang}] stdin write failed");
                break;
            }
            if stdin_handle.flush().is_err() {
                break;
            }
        }
    });

    let lang = config.lang.clone();
    let evt = on_event.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut header_buf = String::new();
        loop {
            header_buf.clear();
            match reader.read_line(&mut header_buf) {
                Ok(0) => break,
                Err(_) => break,
                _ => {}
            }
            let trimmed = header_buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                let content_len: usize = match len_str.trim().parse() {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let mut sep = String::new();
                let _ = reader.read_line(&mut sep);

                let mut body = vec![0u8; content_len];
                if std::io::Read::read_exact(&mut reader, &mut body).is_err() {
                    break;
                }
                let data = String::from_utf8_lossy(&body).to_string();
                let recv_method = extract_method(&data);
                tracing::debug!("[lsp-{lang}] ← {recv_method}");
                if evt.send(LspEvent::Message { data }).is_err() {
                    tracing::warn!("[lsp-{lang}] channel send failed");
                    break;
                }
            }
        }
        let _ = evt.send(LspEvent::Stopped { lang });
    });

    let lang2 = config.lang.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => tracing::debug!("[lsp-{}] {}", lang2, l),
                Err(_) => break,
            }
        }
    });

    let _ = on_event.send(LspEvent::Started {
        lang: config.lang.clone(),
    });

    inner.insert(config.lang, LspProcess { child, stdin_tx });
    Ok(())
}

#[tauri::command]
pub fn lsp_send(
    state: State<LspManager>,
    lang: String,
    message: String,
) -> Result<(), String> {
    let inner = state.inner.lock().map_err(|e| e.to_string())?;
    let proc = inner.get(&lang).ok_or_else(|| format!("no LSP for '{lang}'"))?;
    proc.stdin_tx
        .send(message)
        .map_err(|e| format!("failed to send to LSP: {e}"))
}

#[tauri::command]
pub fn lsp_stop(
    state: State<LspManager>,
    lang: String,
) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    if let Some(mut proc) = inner.remove(&lang) {
        let _ = proc.child.kill();
    }
    Ok(())
}

#[tauri::command]
pub fn lsp_check_available(lang: String) -> bool {
    let (cmd, _) = match find_server(&lang) {
        Some(pair) => pair,
        None => return false,
    };
    #[cfg(not(windows))]
    {
        let resolved = process_util::resolve_command(cmd, None);
        resolved != cmd || std::path::Path::new(cmd).exists()
    }
    #[cfg(windows)]
    {
        true
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonEnvInfo {
    pub python_path: String,
    pub site_packages: Vec<String>,
}

#[tauri::command]
pub fn lsp_detect_python() -> Result<PythonEnvInfo, String> {
    let python = process_util::resolve_command("python3", None);
    let aug_path = process_util::augmented_path(None);
    let output = Command::new(&python)
        .args(["-c", "import sys,site,json;p=list(site.getsitepackages());p.append(site.getusersitepackages());print(json.dumps({'exec':sys.executable,'paths':p}))"])
        .env("PATH", &aug_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("failed to run python3: {e}"))?;
    if !output.status.success() {
        return Ok(PythonEnvInfo {
            python_path: python.to_string(),
            site_packages: vec![],
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("failed to parse python output: {e}"))?;
    let exec_path = parsed["exec"].as_str().unwrap_or(&python).to_string();
    let paths: Vec<String> = parsed["paths"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    Ok(PythonEnvInfo {
        python_path: exec_path,
        site_packages: paths,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonModuleSymbols {
    pub modules: Vec<String>,
    pub symbols: HashMap<String, Vec<String>>,
    pub cached: bool,
}

use std::sync::OnceLock;
use std::time::Instant;

struct PythonModuleCache {
    modules: Vec<String>,
    fetched_at: Instant,
    symbol_cache: HashMap<String, Vec<String>>,
}

static PY_CACHE: OnceLock<Mutex<Option<PythonModuleCache>>> = OnceLock::new();

fn py_cache() -> &'static Mutex<Option<PythonModuleCache>> {
    PY_CACHE.get_or_init(|| Mutex::new(None))
}

fn run_python_script(script: &str, extra_args: &[&str]) -> Option<String> {
    let python = process_util::resolve_command("python3", None);
    let aug_path = process_util::augmented_path(None);
    let mut cmd = Command::new(&python);
    cmd.args(["-c", script])
        .env("PATH", &aug_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for a in extra_args {
        cmd.arg(a);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub fn lsp_python_env_symbols(modules: Vec<String>) -> Result<PythonModuleSymbols, String> {
    let mut guard = py_cache().lock().map_err(|e| e.to_string())?;
    let now = Instant::now();

    let cache_valid = guard.as_ref().map_or(false, |c| now.duration_since(c.fetched_at).as_secs() < 300);

    let all_modules = if cache_valid {
        guard.as_ref().unwrap().modules.clone()
    } else {
        let script = "import json,pkgutil;print(json.dumps(sorted(set(m.name for m in pkgutil.iter_modules() if not m.name.startswith('_')))))";
        let mods: Vec<String> = run_python_script(script, &[])
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let c = guard.get_or_insert_with(|| PythonModuleCache {
            modules: vec![],
            fetched_at: now,
            symbol_cache: HashMap::new(),
        });
        c.modules = mods.clone();
        c.fetched_at = now;
        mods
    };

    let mut need_fetch: Vec<String> = Vec::new();
    let mut symbols: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(ref c) = *guard {
        for m in &modules {
            if let Some(cached) = c.symbol_cache.get(m) {
                symbols.insert(m.clone(), cached.clone());
            } else {
                need_fetch.push(m.clone());
            }
        }
    } else {
        need_fetch = modules.clone();
    }

    if !need_fetch.is_empty() {
        let script = r#"import json,sys,importlib
r={}
for n in sys.argv[1:]:
 try:
  m=importlib.import_module(n);r[n]=[a for a in dir(m) if not a.startswith('_')][:500]
 except: pass
print(json.dumps(r))"#;
        let args: Vec<&str> = need_fetch.iter().map(|s| s.as_str()).collect();
        if let Some(out) = run_python_script(script, &args) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&out) {
                if let Some(obj) = parsed.as_object() {
                    let c = guard.get_or_insert_with(|| PythonModuleCache {
                        modules: vec![],
                        fetched_at: now,
                        symbol_cache: HashMap::new(),
                    });
                    for (k, v) in obj {
                        if let Some(arr) = v.as_array() {
                            let syms: Vec<String> =
                                arr.iter().filter_map(|s| s.as_str().map(String::from)).collect();
                            c.symbol_cache.insert(k.clone(), syms.clone());
                            symbols.insert(k.clone(), syms);
                        }
                    }
                }
            }
        }
    }

    Ok(PythonModuleSymbols {
        modules: all_modules,
        symbols,
        cached: cache_valid,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEnvSymbols {
    pub packages: Vec<String>,
    pub exports: HashMap<String, Vec<String>>,
}

#[tauri::command]
pub fn lsp_node_env_symbols(project_dir: String, modules: Vec<String>) -> Result<NodeEnvSymbols, String> {
    let node = process_util::resolve_command("node", None);
    let aug_path = process_util::augmented_path(None);

    let mut packages = Vec::new();
    let pkg_path = std::path::Path::new(&project_dir).join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            for section in &["dependencies", "devDependencies"] {
                if let Some(obj) = parsed[section].as_object() {
                    for k in obj.keys() {
                        if !k.starts_with('@') || k.contains('/') {
                            packages.push(k.clone());
                        }
                    }
                }
            }
        }
    }

    let node_mods = std::path::Path::new(&project_dir).join("node_modules");
    if packages.is_empty() && node_mods.exists() {
        if let Ok(entries) = std::fs::read_dir(&node_mods) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') && !name.starts_with('_') {
                    packages.push(name);
                }
            }
        }
    }

    let mut exports: HashMap<String, Vec<String>> = HashMap::new();
    if !modules.is_empty() {
        let script = format!(
            r#"const r={{}};for(const n of process.argv.slice(1)){{try{{const m=require(n);r[n]=Object.getOwnPropertyNames(m).filter(k=>!k.startsWith('_')).slice(0,500)}}catch{{}}}};console.log(JSON.stringify(r))"#
        );
        let mut cmd = Command::new(&node);
        cmd.args(["-e", &script])
            .env("PATH", &aug_path)
            .env("NODE_PATH", node_mods.to_str().unwrap_or(""))
            .current_dir(&project_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for m in &modules {
            cmd.arg(m);
        }
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
                    if let Some(obj) = parsed.as_object() {
                        for (k, v) in obj {
                            if let Some(arr) = v.as_array() {
                                exports.insert(
                                    k.clone(),
                                    arr.iter().filter_map(|s| s.as_str().map(String::from)).collect(),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(NodeEnvSymbols { packages, exports })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoEnvSymbols {
    pub packages: Vec<String>,
}

#[tauri::command]
pub fn lsp_go_env_symbols(project_dir: String) -> Result<GoEnvSymbols, String> {
    let go_cmd = process_util::resolve_command("go", None);
    let aug_path = process_util::augmented_path(None);

    let output = Command::new(&go_cmd)
        .args(["list", "-m", "all"])
        .env("PATH", &aug_path)
        .current_dir(&project_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let mut packages = Vec::new();
    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if let Some(pkg) = parts.first() {
                    let name = pkg.rsplit('/').next().unwrap_or(pkg);
                    if !name.is_empty() && !name.starts_with('_') {
                        packages.push(name.to_string());
                    }
                }
            }
        }
    }

    let output2 = Command::new(&go_cmd)
        .args(["list", "std"])
        .env("PATH", &aug_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    if let Ok(out) = output2 {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let name = line.rsplit('/').next().unwrap_or(line);
                if !name.is_empty() && !name.starts_with('_') && !name.contains("internal") {
                    packages.push(name.to_string());
                }
            }
        }
    }

    packages.sort();
    packages.dedup();
    Ok(GoEnvSymbols { packages })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LangEnvSymbols {
    pub symbols: Vec<String>,
    pub api_symbols: HashMap<String, Vec<String>>,
}

fn run_cmd_collect(cmd_name: &str, args: &[&str], cwd: Option<&str>) -> Vec<String> {
    let resolved = process_util::resolve_command(cmd_name, None);
    let aug_path = process_util::augmented_path(None);
    let mut cmd = Command::new(&resolved);
    cmd.args(args)
        .env("PATH", &aug_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.trim().to_string())
                .collect()
        }
        _ => vec![],
    }
}

#[tauri::command]
pub fn lsp_lang_env_symbols(lang: String, project_dir: String, modules: Vec<String>) -> Result<LangEnvSymbols, String> {
    let mut symbols = Vec::new();
    let mut api_symbols: HashMap<String, Vec<String>> = HashMap::new();

    match lang.as_str() {
        "lua" => {
            let script = r#"
local r={}
for k,_ in pairs(package.loaded) do r[#r+1]=k end
for k,_ in pairs(_G) do if type(k)=="string" and not k:match("^_") then r[#r+1]=k end end
table.sort(r)
for _,v in ipairs(r) do print(v) end"#;
            symbols = run_cmd_collect("lua", &["-e", script.trim()], None);
            if symbols.is_empty() {
                symbols = run_cmd_collect("lua5.4", &["-e", script.trim()], None);
            }
            if symbols.is_empty() {
                symbols = run_cmd_collect("luajit", &["-e", script.trim()], None);
            }
            for m in &modules {
                let mod_script = format!(
                    "local ok,mod=pcall(require,'{}');if ok and type(mod)=='table' then for k,_ in pairs(mod) do if type(k)=='string' and not k:match('^_') then print(k) end end end",
                    m
                );
                let syms = run_cmd_collect("lua", &["-e", &mod_script], None);
                if !syms.is_empty() {
                    api_symbols.insert(m.clone(), syms);
                }
            }
        }
        "ruby" => {
            let lines = run_cmd_collect("ruby", &["-e", "puts Gem::Specification.map(&:name).sort.uniq"], None);
            symbols.extend(lines);
            let builtins = run_cmd_collect("ruby", &["-e", "puts Object.constants.sort"], None);
            symbols.extend(builtins);
            for m in &modules {
                let script = format!(
                    "begin;require '{}';m=Object.const_get('{}');puts m.instance_methods(false).sort rescue puts m.public_methods(false).sort;rescue=>e;end",
                    m, m.chars().next().unwrap_or('X').to_uppercase().to_string() + &m[1..]
                );
                let syms = run_cmd_collect("ruby", &["-e", &script], None);
                if !syms.is_empty() {
                    api_symbols.insert(m.clone(), syms);
                }
            }
        }
        "php" => {
            let exts = run_cmd_collect("php", &["-m"], None);
            symbols.extend(exts.iter().filter(|e| !e.starts_with('[')).cloned());
            let fns = run_cmd_collect("php", &["-r", "echo implode(\"\\n\",array_slice(get_defined_functions()['internal'],0,500));"], None);
            symbols.extend(fns);
        }
        "dart" => {
            let deps = run_cmd_collect("dart", &["pub", "deps", "--style=compact"], Some(&project_dir));
            for line in &deps {
                if let Some(name) = line.split_whitespace().next() {
                    if name.chars().next().map_or(false, |c| c.is_alphabetic()) {
                        symbols.push(name.to_string());
                    }
                }
            }
        }
        "kotlin" | "java" => {
            let script = r#"import java.util.jar.*;import java.io.*;public class _Ls{public static void main(String[] a){for(String p:System.getProperty("java.class.path","").split(File.pathSeparator)){try{JarFile j=new JarFile(p);j.stream().filter(e->e.getName().endsWith(".class")).forEach(e->{String n=e.getName().replace('/','.');n=n.substring(0,n.length()-6);String s=n.contains(".")?n.substring(n.lastIndexOf('.')+1):n;if(!s.isEmpty()&&!s.startsWith("_"))System.out.println(s);});j.close();}catch(Exception ex){}}}}"#;
            let _ = script;
            let common = vec![
                "String","Integer","Long","Double","Float","Boolean","Character","Byte","Short",
                "ArrayList","LinkedList","HashMap","TreeMap","HashSet","TreeSet","LinkedHashMap",
                "Collections","Arrays","Objects","Optional","Stream","Collectors",
                "List","Map","Set","Queue","Deque","Iterator","Iterable","Comparable",
                "Thread","Runnable","Callable","Future","CompletableFuture","ExecutorService",
                "IOException","Exception","RuntimeException","NullPointerException",
                "StringBuilder","StringBuffer","Scanner","Random","BigDecimal","BigInteger",
                "File","Path","Paths","Files","InputStream","OutputStream","Reader","Writer",
                "BufferedReader","BufferedWriter","FileReader","FileWriter","PrintWriter",
                "Socket","ServerSocket","URL","URI","HttpURLConnection",
                "Pattern","Matcher","DateTimeFormatter","LocalDate","LocalDateTime","Instant",
                "System","Math","Class","Object","Enum","Annotation","Override","Deprecated",
            ];
            symbols.extend(common.into_iter().map(String::from));
        }
        "swift" => {
            let common = vec![
                "String","Int","Double","Float","Bool","Array","Dictionary","Set","Optional",
                "print","debugPrint","fatalError","precondition","assert",
                "struct","class","enum","protocol","extension","func","var","let","guard",
                "UIView","UIViewController","UILabel","UIButton","UITableView","UICollectionView",
                "URLSession","URLRequest","JSONDecoder","JSONEncoder","Codable","Decodable","Encodable",
                "DispatchQueue","OperationQueue","NotificationCenter","UserDefaults","Bundle",
                "CGFloat","CGPoint","CGSize","CGRect","NSObject","NSError",
                "SwiftUI","View","Text","Button","NavigationView","List","VStack","HStack","ZStack",
                "State","Binding","ObservableObject","Published","EnvironmentObject",
            ];
            symbols.extend(common.into_iter().map(String::from));
        }
        _ => {}
    }

    symbols.sort();
    symbols.dedup();
    Ok(LangEnvSymbols { symbols, api_symbols })
}

#[derive(Serialize)]
pub struct LspInfo {
    lang: String,
    running: bool,
}

#[tauri::command]
pub fn lsp_list(state: State<LspManager>) -> Result<Vec<LspInfo>, String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    prune_stopped(&mut inner);
    let known: Vec<&str> = KNOWN_SERVERS.iter().map(|(l, _, _)| *l).collect();
    let mut out: Vec<LspInfo> = known
        .into_iter()
        .map(|l| LspInfo {
            lang: l.to_string(),
            running: inner.contains_key(l),
        })
        .collect();
    for key in inner.keys() {
        if !out.iter().any(|i| i.lang == *key) {
            out.push(LspInfo {
                lang: key.clone(),
                running: true,
            });
        }
    }
    Ok(out)
}
