mod ai;
mod auth;
mod browser;
mod capture;
mod computer;
mod debug;
mod devin;
mod extensions;
mod files;
mod git;
mod lsp;
mod marketplace;
mod net;
mod process_util;
mod tasks;
mod terminal;
mod watcher;

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
}

/// Entry point shared by the binary and (potentially) mobile targets.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_macos_fps::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

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
        .manage(watcher::WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            files::register_workspace_root,
            files::read_dir,
            files::read_text_file,
            files::write_text_file,
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
            git::git_file_head,
            git::git_diff,
            git::git_stage,
            git::git_unstage,
            git::git_stage_all,
            git::git_unstage_all,
            git::git_commit,
            git::git_push,
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
            ai::web_fetch,
            ai::web_search,
            net::http_request,
            net::download_file,
            capture::capture_url,
            browser::browser_navigate,
            browser::browser_click,
            browser::browser_type,
            browser::browser_press,
            browser::browser_eval,
            browser::browser_screenshot,
            browser::browser_close,
            computer::computer_screenshot,
            computer::computer_move,
            computer::computer_click,
            computer::computer_double_click,
            computer::computer_type,
            computer::computer_key,
            computer::computer_scroll,
            devin::devin_create_session,
            devin::devin_send_message,
            devin::devin_get_session,
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
            }
        });
}
