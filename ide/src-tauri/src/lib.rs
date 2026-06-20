mod ai;
mod debug;
mod extensions;
mod files;
mod git;
mod lsp;
mod marketplace;
mod tasks;
mod terminal;
mod watcher;

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
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .manage(terminal::TerminalState::default())
        .manage(lsp::LspManager::default())
        .manage(debug::DebugManager::default())
        .manage(watcher::WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            files::read_dir,
            files::read_text_file,
            files::write_text_file,
            files::home_dir,
            files::create_file,
            files::create_dir,
            files::rename_path,
            files::delete_path,
            files::search_in_project,
            files::replace_in_file,
            files::replace_in_project,
            git::git_status,
            git::git_file_head,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Michael IDE");
}
