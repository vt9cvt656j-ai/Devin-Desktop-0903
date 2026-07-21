// Platform accessibility helpers are retained as a fallback for the desktop
// automation bridge; not every target currently wires each helper into a command.
#[allow(dead_code)]
mod accessibility;
mod ai;
mod auth;
mod automation;
mod browser;
mod capture;
mod db;
mod debug;
mod extensions;
mod files;
mod game;
mod game_assets;
mod git;
mod image_location;
mod knowledge;
mod local_discovery;
mod location;
mod lsp;
mod marketplace;
mod mcp;
mod net;
mod process_util;
mod proxy;
mod public_data;
mod qr;
mod shop_catalog;
mod sysctl;
mod tasks;
mod terminal;
mod watcher;
mod web_scaffold;

// DevTools remain compiled in for manual use, but the application window must
// stay focused on startup instead of forcing the inspector open.
#[cfg(desktop)]
fn should_open_devtools_on_startup() -> bool {
    false
}

/// Reap any backend processes left over from a previous page session. The
/// frontend calls this once on startup, so a webview reload (common during dev)
/// no longer orphans the old terminals / LSP servers / debug adapters — the main
/// cause of "the IDE gets slower and freezes after running a while".
#[tauri::command]
fn cleanup_stale(
    term: tauri::State<terminal::TerminalState>,
    lsp: tauri::State<lsp::LspManager>,
    dap: tauri::State<debug::DebugManager>,
) {
    term.reset_all();
    lsp.stop_all();
    dap.stop_all();
    mcp::stop_all();
    browser::close_all();
}

/// Entry point shared by the binary and (potentially) mobile targets.
/// Crash reporting: append every Rust panic (message + location + backtrace + timestamp)
/// to `~/.michael-ide/crash.log`, so a crash on a user's machine leaves a trace we can ask
/// them for — instead of the window just vanishing with nothing to go on. Chains the
/// previous hook so normal panic printing still happens.
fn install_panic_logger() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            let dir = std::path::PathBuf::from(home).join(".michael-ide");
            let _ = std::fs::create_dir_all(&dir);
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let loc = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_default();
            let msg = format!(
                "\n===== PANIC (unix {ts}) =====\n{info}\nat: {loc}\n{:#?}\n",
                std::backtrace::Backtrace::force_capture()
            );
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("crash.log"))
            {
                let _ = f.write_all(msg.as_bytes());
            }
        }
        prev(info);
    }));
}

pub fn run() {
    install_panic_logger();
    browser::kill_orphaned_browsers();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::new().build());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_plugin_macos_fps::init());
    }

    builder
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            // open_devtools only exists in debug builds or with the opt-in `devtools`
            // feature — release builds no longer compile the inspector in at all, so a
            // shipped app can't be opened up to read code / watch authorized requests.
            #[cfg(all(desktop, any(debug_assertions, feature = "devtools")))]
            if should_open_devtools_on_startup() {
                use tauri::Manager;
                if let Some(w) = app.get_webview_window("main") {
                    w.open_devtools();
                }
            }

            files::bootstrap_home_root();

            tauri::async_runtime::spawn(async {
                if let Err(e) = auth::init_db().await {
                    tracing::warn!("Auth DB init skipped: {e}");
                }
            });

            Ok(())
        })
        .manage(terminal::TerminalState::default())
        .manage(lsp::LspManager::default())
        .manage(debug::DebugManager::default())
        .manage(proxy::ProxyState::default())
        .manage(watcher::WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            files::register_workspace_root,
            files::read_dir,
            files::read_text_file,
            files::read_log_tail,
            files::read_file_data_url,
            files::inspect_file,
            files::write_text_file,
            files::write_text_file_if_unchanged,
            files::delete_text_file_if_unchanged,
            files::write_tmp_file,
            files::home_dir,
            files::create_file,
            files::create_dir,
            files::rename_path,
            files::copy_path,
            files::delete_path,
            files::search_in_project,
            files::replace_in_file,
            files::replace_in_project,
            git::git_status,
            git::git_worktree_add,
            git::git_worktree_list,
            git::git_worktree_remove,
            git::git_file_head,
            git::git_diff,
            git::git_stage,
            git::git_unstage,
            git::git_stage_all,
            git::git_unstage_all,
            git::git_commit,
            git::git_push,
            git::git_clone,
            git::git_branches,
            git::git_checkout,
            git::git_pull,
            git::git_conflicts,
            git::git_merge_versions,
            git::git_resolve_conflict,
            git::git_log,
            git::git_stash,
            git::git_stash_pop,
            git::git_stash_apply,
            git::git_stash_drop,
            git::git_stash_list,
            git::git_blame,
            ai::ai_chat,
            ai::ai_chat_with_tools,
            ai::ai_complete,
            ai::cancel_ai,
            ai::web_fetch,
            ai::web_search,
            net::http_request,
            net::tor_request,
            net::download_file,
            net::generate_image_chat,
            proxy::proxy_available,
            proxy::proxy_status,
            proxy::proxy_start,
            proxy::proxy_stop,
            proxy::proxy_ca_path,
            proxy::proxy_set_system_proxy,
            qr::decode_qr,
            image_location::reverse_geocode_coordinates,
            location::request_current_location,
            db::db_query,
            mcp::mcp_connect,
            mcp::mcp_call,
            mcp::mcp_status,
            mcp::mcp_disconnect,
            capture::capture_url,
            capture::capture_url_frames,
            browser::browser_navigate,
            browser::browser_click,
            browser::browser_type,
            files::read_document,
            browser::browser_press,
            browser::browser_upload_file,
            browser::browser_eval,
            browser::browser_screenshot,
            browser::browser_set_viewport,
            browser::browser_set_marks,
            browser::browser_scroll,
            browser::browser_wait,
            browser::browser_cookies,
            browser::browser_storage,
            browser::browser_close,
            sysctl::system_open_app,
            sysctl::system_list_apps,
            sysctl::system_app_windows,
            sysctl::system_focus_window,
            sysctl::system_menu,
            sysctl::system_menu_items,
            sysctl::system_frontmost,
            extensions::ext_list_installed,
            extensions::ext_read_asset,
            extensions::ext_set_enabled,
            extensions::ext_uninstall,
            extensions::ext_available_builtin,
            extensions::ext_install_builtin,
            extensions::ext_install_from_path,
            terminal::term_open,
            terminal::term_write,
            terminal::term_resize,
            terminal::term_close,
            terminal::term_list_commands,
            terminal::term_history,
            lsp::lsp_start,
            lsp::lsp_send,
            lsp::lsp_stop,
            lsp::lsp_list,
            lsp::lsp_check_available,
            lsp::lsp_detect_python,
            lsp::lsp_python_env_symbols,
            lsp::lsp_node_env_symbols,
            lsp::lsp_go_env_symbols,
            lsp::lsp_lang_env_symbols,
            debug::dap_start,
            debug::dap_send,
            debug::dap_stop,
            debug::dap_list,
            marketplace::marketplace_list,
            marketplace::marketplace_install,
            marketplace::marketplace_search,
            tasks::tasks_list,
            tasks::task_run_capture,
            watcher::fs_watch,
            watcher::fs_unwatch,
            auth::auth_login_or_register,
            auth::auth_login,
            auth::auth_register,
            auth::auth_check_email,
            auth::auth_send_code,
            auth::auth_verify_code,
            auth::db_marketplace_list,
            auth::db_marketplace_upsert,
            accessibility::read_screen,
            accessibility::ui_click,
            automation::automation_call,
            knowledge::academic_search,
            knowledge::package_search,
            knowledge::github_search,
            knowledge::github_repo,
            knowledge::gitlab_repo,
            knowledge::gitee_repo,
            knowledge::codeberg_repo,
            knowledge::cve_search,
            knowledge::wiki_search,
            knowledge::stackoverflow_search,
            knowledge::hackernews_search,
            knowledge::developer_community_search,
            knowledge::dockerhub_search,
            knowledge::pubmed_search,
            knowledge::arxiv_search,
            knowledge::crossref_search,
            knowledge::openalex_search,
            knowledge::pubchem_search,
            knowledge::clinical_trials_search,
            knowledge::gitlab_search,
            knowledge::gitee_search,
            knowledge::maven_search,
            knowledge::packagist_search,
            knowledge::rubygems_search,
            knowledge::nuget_search,
            knowledge::homebrew_search,
            knowledge::mdn_search,
            knowledge::cdnjs_search,
            knowledge::bundlephobia_search,
            knowledge::devto_search,
            knowledge::reddit_search,
            knowledge::smzdm_search,
            knowledge::xianyu_search,
            knowledge::zhuanzhuan_search,
            knowledge::steam_search,
            knowledge::iconify_search,
            knowledge::color_search,
            knowledge::lobsters_search,
            knowledge::juejin_search,
            knowledge::codrops_search,
            knowledge::smashingmag_search,
            knowledge::css_tricks_search,
            knowledge::codepen_search,
            knowledge::dribbble_search,
            knowledge::awwwards_search,
            knowledge::v2ex_search,
            knowledge::segmentfault_search,
            knowledge::github_discussions_search,
            knowledge::producthunt_search,
            knowledge::freecodecamp_search,
            knowledge::github_trending,
            knowledge::infoq_search,
            knowledge::hackernoon_search,
            knowledge::codeberg_search,
            knowledge::bestofjs_search,
            knowledge::sourcegraph_search,
            knowledge::deep_search,
            local_discovery::local_discovery,
            public_data::live_environment,
            public_data::live_markets,
            public_data::live_flights,
            public_data::road_environment,
            public_data::track_shipment,
            shop_catalog::shop_catalog,
            game::game_scaffold,
            web_scaffold::web_scaffold,
            game_assets::generate_3d,
            game_assets::generate_sound,
            game_assets::generate_music,
            game_assets::generate_voice,
            game_assets::auto_rig,
            game_assets::generate_motion,
            game_assets::generate_texture,
            game_assets::search_game_assets,
            game_assets::download_asset,
            cleanup_stale,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Michael IDE")
        .run(|handle, event| {
            // On app exit, kill every child process (shells / LSP servers / debug
            // adapters) so nothing is left running after the window closes.
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                use tauri::Manager;
                handle.state::<terminal::TerminalState>().reset_all();
                handle.state::<lsp::LspManager>().stop_all();
                handle.state::<debug::DebugManager>().stop_all();
                proxy::stop_all(&handle.state::<proxy::ProxyState>()); // reap the mitmdump proxy
                mcp::stop_all(); // reap MCP servers on quit (global map, not Tauri State)
                automation::stop(); // reap the desktop-automation server
            }
        });
}

#[cfg(all(test, desktop))]
mod tests {
    use super::should_open_devtools_on_startup;

    #[test]
    fn devtools_stay_closed_on_startup() {
        assert!(!should_open_devtools_on_startup());
    }
}
